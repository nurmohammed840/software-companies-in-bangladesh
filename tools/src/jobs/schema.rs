use super::*;

pub type Jobs = Map<String, Vec<JobPost>>;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobPost {
    pub title: String,

    /// Job description formatted as Markdown.
    pub description: String,

    pub employment_type: Option<EmploymentType>,
    /// Job role or seniority.
    pub role: Option<String>,

    pub posted_at: Option<PostedAt>,

    /// Application deadline. Use `Expired` only if the posting explicitly states
    /// that applications are closed or expired.
    pub deadline: Option<Deadline>,

    pub location: Option<JobLocation>,

    /// Required or preferred experience.
    pub experience: Option<String>,

    pub salary: Option<Salary>,

    /// Number of open positions.
    pub vacancies: Option<u32>,

    // Relevant technologies, skills, tools
    pub tags: Vec<String>,

    /// Ways to apply.
    pub apply: Vec<ApplicationMethod>,

    /// Original job posting link.
    /// Include only when found. Keep the URL exactly as provided; Never guess.
    pub source: Option<String>,

    /// `true` if `source` should be fetched for complete job details.
    pub needs_fetch: bool,

    #[schemars(range(min = 0.0, max = 1.0))]
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Salary {
    /// Minimum salary.
    pub min: Option<f64>,

    /// Maximum salary.
    pub max: Option<f64>,

    /// ISO 4217 currency code (e.g. "USD", "BDT", "EUR").
    pub currency: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum Deadline {
    /// Application deadline.
    Date(PostedAt),

    /// Applications are closed.
    Expired,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum PostedAt {
    Absolute(String),
    /// Relative time, e.g. "2 days ago".
    Relative(String),
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum ApplicationMethod {
    Email(String),
    Website(String),
}

impl ApplicationMethod {
    pub fn website(&self) -> Option<&str> {
        match self {
            ApplicationMethod::Email(_) => None,
            ApplicationMethod::Website(url) => Some(url),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum EmploymentType {
    FullTime,
    PartTime,
    Contract,
    Temporary,
    Internship,
    Freelance,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum JobLocation {
    Remote,
    Hybrid(String),
    OnSite(String),
}
