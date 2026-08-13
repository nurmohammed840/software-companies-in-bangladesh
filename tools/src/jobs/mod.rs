//! Website
//!   │
//!   ▼
//! HTML
//!   │
//!   ▼
//! Markdown (Normalize)
//!   │
//!   ▼
//! LLM Extraction (Input: Instructions + JSON Schema + Markdown)
//!   │
//!   ▼
//! JobPost[]
//!   │
//!   ▼
//! Store on Disk
mod input_normalizer;
pub mod schema;

use input_normalizer::normalize_markdown_from;
use schema::*;

use std::collections::BTreeMap as Map;
use std::{io::Write, path::Path};

use crate::data::Company;
use crate::utils::resolve_url;
use crate::utils::{cache::Cache, normalize_url, text_file::*};
use crate::{Result, data::Companies};
use chromiumoxide::{Browser, BrowserConfig};
use futures::{StreamExt, stream};
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, JsonSpec};
use log::{error, info, warn};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json as json;
use url::Url;

const LLM_INPUT: &str = r#"You are an information extraction engine.

Your task is to extract job postings.

Rules:
- Extract only actual job postings. If no job posting is found, return `[]`.
- Set `needsFetch` to `true` if fetching `source` is required to extract the complete job.
- Never guess. use schema defaults when required.
- Keep `title` unchanged.
- Exclude on-site or location-specific jobs confirmed to be outside Bangladesh.
- Remote jobs may be included even outside Bangladesh.
- Format `description` as Markdown. Reorganize if needed, but do not add or remove information.
- Extract relevant tags from the `description` when possible and add them to `tags`; do not invent or duplicate tags.
- If found, preserve `source` exactly as provided; never resolve to absolute URL, normalize, or modify it.
- Never guess salary information. Only extract salary information that is explicitly provided in the source.
- Never guess, infer, or generate unrealistic values for any fields in schema.
- Include a confidence score (0.0–1.0) for each extracted job.
- Do not include job postings with confidence below `0.5`
"#;

#[tokio::main]
pub async fn run(
    model: String,
    dir: &Path,
    companies: &Companies<'_>,
    log_file: bool,
    concurrent: u8,
) -> Result {
    let mut engine = Crawler::new().await?.model(model);

    // let jobs = engine.fetch_jobs(&Url::parse("https://therap.hire.trakstar.com/")?).await?;
    // println!("{}", json::to_string_pretty(&jobs).unwrap());
    // return Ok(());

    if log_file {
        engine.with_log(&dir.join("job-postings.pages.md"))?;
    }

    let output_file = TextFile::read(dir.join("./data/job-posts.json"))?;

    let mut jobs = stream::iter(companies.iter())
        .map(|(name, company)| engine.fetch_jobs_from(name, company))
        .buffered(concurrent.into());

    let mut output = Jobs::new();
    while let Some(result) = jobs.next().await {
        match result {
            Ok(None) => {}
            Ok(Some((name, source, jobs))) => {
                output.insert(name, Entry { source, jobs });
            }
            Err(msg) => {
                error!("[ERROR]: {msg}");
                break;
            }
        }
    }

    drop(jobs);

    info!(
        "From {} companies; Found {} Jobs",
        output.len(),
        output.values().map(|f| f.jobs.len()).sum::<usize>()
    );
    output_file.write(json::to_string_pretty(&output)?)?;

    engine.close().await?;
    Ok(())
}

const CACHE_PATH: &str = "job-postings-cache";

pub fn clear_cache() -> Result {
    Cache::clear(CACHE_PATH)
}

struct Crawler {
    browser: Browser,
    llm_model: String,
    log_file: Option<LogFile>,
}

impl Crawler {
    async fn fetch_jobs_from(
        &self,
        name: &str,
        company: &Company,
    ) -> Result<Option<(String, Url, Vec<JobPost>)>> {
        let Some(url) = company.links.job.as_ref() else {
            return Ok(None);
        };
        let url = normalize_url(url)?;
        let jobs = self.fetch_jobs(&url).await?;
        Ok(Some((name.into(), url, jobs)))
    }

    async fn fetch_jobs(&self, url: &Url) -> Result<Vec<JobPost>> {
        let mut jobs = self.get_cached_or_fetch_jobs(url).await?;

        for job in jobs.iter_mut().filter(|job| job.needs_fetch) {
            let urls = job
                .source
                .as_deref()
                .into_iter()
                .chain(job.apply.iter().find_map(|m| m.website()));

            let Some(new_url) = find_resolved_url(url, urls)? else {
                info!("[NO-SRC] {}; FROM: {}", job.title, url);
                continue;
            };

            let Some(post) = self.fetch_job_post(url, &new_url).await? else {
                continue;
            };

            *job = post;
        }

        Ok(jobs)
    }

    async fn get_cached_or_fetch_jobs(&self, url: &Url) -> Result<Vec<JobPost>> {
        let url = normalize_url(&url)?;
        let cache = Cache::open(CACHE_PATH, url.as_str())?;

        match cache.get()? {
            Some(json) => Ok(json::from_str(&json)?),
            None => {
                let json = self.fetch_llm_json_output(&url).await?;
                let data = json::from_str(&json).map_err(|err| format!("{err};\n{json}"))?;
                cache.set(&json)?;
                Ok(data)
            }
        }
    }

    async fn fetch_job_post(&self, from: &Url, url: &Url) -> Result<Option<JobPost>> {
        let mut jobs = self.get_cached_or_fetch_jobs(url).await?;

        if jobs.is_empty() {
            return Ok(None);
        }

        if jobs.len() > 1 {
            warn!("[MULTIPLE-POST]: {from} -> {url}");
            return Ok(None);
        }

        let post = jobs.pop().unwrap();

        if post.needs_fetch {
            info!("[NEED-FETCH]: {from} -> {url}");
            info!(
                "Title: {}; Score: {}; Src: {:?}",
                post.title, post.confidence, post.source
            );
            return Ok(None);
        }

        Ok(Some(post))
    }

    async fn fetch(&self, url: &Url) -> Result<String> {
        let page = self.browser.new_page(url.clone()).await?;

        page.wait_for_navigation()
            .await?
            .evaluate("new Promise(resolve => setTimeout(resolve, 7 * 1000))") // Execute JS
            .await?;

        let html = page.content().await?;

        page.close().await?;

        Ok(html)
    }

    async fn fetch_llm_json_output(&self, url: &Url) -> Result<String> {
        let html = self.fetch(url).await?;
        let markdown = normalize_markdown_from(&html)?;

        if let Some(file) = &self.log_file {
            writeln!(file.as_ref(), "---\n{url}\n{markdown}")?;
        }

        info!("[LLM-CALL]: {url}");

        let client = genai::Client::default();
        let schema = json::to_value(schema_for!(Vec<JobPost>))?;
        let schema = JsonSpec::new("JobPosts", schema);
        // let model = ModelIden::new(AdapterKind::Groq, self.llm_model.clone());

        let options = ChatOptions::default()
            .with_temperature(0.0)
            .with_response_format(schema);

        let req = ChatRequest::new(vec![
            ChatMessage::system(LLM_INPUT.to_string()),
            ChatMessage::user(format!("Extract from this markdown.\n\n{markdown}")),
        ]);

        let res = client
            .exec_chat(&self.llm_model, req, Some(&options))
            .await?;

        Ok(res.texts().join(""))
    }
}

fn find_resolved_url<'a>(
    base: &Url,
    urls: impl IntoIterator<Item = &'a str>,
) -> Result<Option<Url>> {
    for src in urls {
        // println!("src: {}", src);

        let resolved = resolve_url(base, src)?;
        if &resolved != base {
            return Ok(Some(resolved));
        }
    }
    Ok(None)
}

impl Crawler {
    async fn new() -> Result<Self> {
        let (browser, mut handler) = Browser::launch(
            BrowserConfig::builder()
                .new_headless_mode()
                // .with_head()
                .build()?,
        )
        .await?;

        tokio::spawn(async move {
            while let Some(_ev) = handler.next().await {
                // ...
            }
        });

        Ok(Self {
            browser,
            llm_model: "gemini-3.5-flash-lite".into(),
            log_file: None,
        })
    }

    fn with_log(&mut self, path: &Path) -> Result<&mut Self> {
        self.log_file = Some(open_log_file(path)?);
        return Ok(self);
    }

    fn model(mut self, llm_model: String) -> Self {
        self.llm_model = llm_model;
        return self;
    }

    async fn close(&mut self) -> Result {
        self.browser.close().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::utils::cache::*;

    #[allow(unused_imports)]
    use super::*;
    use genai::{Client, adapter::AdapterKind};

    /// Run: `cargo test -- --nocapture`
    #[tokio::test]
    async fn log_llm_models() {
        let models = Client::default()
            .all_model_names(AdapterKind::Gemini, None)
            .await;

        println!("models: {models:#?}");
    }

    // #[test]
    // fn remove_from_cache() -> Result {
    //     let pat = "flytesolutions";
    //     let dir = tmp_cache_dir(CACHE_PATH)?;

    //     for entry in fs::read_dir(&dir)? {
    //         let path = entry?.path();
    //         if path.is_file()
    //             && path
    //                 .file_name()
    //                 .and_then(|name| name.to_str())
    //                 .is_some_and(|name| name.contains(pat))
    //         {
    //             fs::remove_file(&path)?;
    //             println!("cached removed: {path:?}");
    //         }
    //     }
    //     Ok(())
    // }

    #[test]
    fn log_cache() -> Result {
        let dir = tmp_cache_dir(CACHE_PATH)?;
        let count = fs::read_dir(&dir)?.count();

        println!("cache dir: {:?}; entries: {}", dir, count);
        Ok(())
    }
}
