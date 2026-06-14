use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

#[derive(Debug, sqlx::FromRow)]
pub struct DokployProject {
    #[sqlx(rename = "projectId")]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DokployApplication {
    #[sqlx(rename = "applicationId")]
    pub id: String,
    pub name: String,
    #[sqlx(rename = "sourceType")]
    pub source_type: String, // github, gitea, git
    #[sqlx(rename = "repository")]
    pub repository: Option<String>,
    #[sqlx(rename = "owner")]
    pub owner: Option<String>,
    #[sqlx(rename = "branch")]
    pub branch: Option<String>,
    #[sqlx(rename = "buildPath")]
    pub build_path: Option<String>,
    #[sqlx(rename = "customGitUrl")]
    pub custom_git_url: Option<String>,
    #[sqlx(rename = "customGitBranch")]
    pub custom_git_branch: Option<String>,
    #[sqlx(rename = "buildType")]
    pub build_type: String, // dockerfile, nixpacks, heroku_buildpacks
    pub env: Option<String>, // KEY=VAL\nKEY2=VAL2
    #[sqlx(rename = "projectId")]
    pub project_id: String,
    // Add gitea specific fields if needed
    #[sqlx(rename = "giteaRepository")]
    pub gitea_repository: Option<String>,
    #[sqlx(rename = "giteaOwner")]
    pub gitea_owner: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DokployCompose {
    #[sqlx(rename = "composeId")]
    pub id: String,
    pub name: String,
    #[sqlx(rename = "composeFile")]
    pub compose_file: String,
    #[sqlx(rename = "projectId")]
    pub project_id: String,
    pub env: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DokployDomain {
    pub host: String,
    pub https: bool,
    pub port: i32,
    #[sqlx(rename = "applicationId")]
    pub application_id: Option<String>,
    #[sqlx(rename = "composeId")]
    pub compose_id: Option<String>,
}

pub struct DokployData {
    pub projects: Vec<DokployProject>,
    pub applications: Vec<DokployApplication>,
    pub composes: Vec<DokployCompose>,
    pub domains: Vec<DokployDomain>,
}

pub struct DokploySource {
    pool: Pool<Postgres>,
}

impl DokploySource {
    pub async fn new(pg_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(pg_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn fetch_all(&self) -> Result<DokployData> {
        let projects = sqlx::query_as::<_, DokployProject>("SELECT \"projectId\", name, description FROM \"project\"")
            .fetch_all(&self.pool)
            .await?;

        let applications = sqlx::query_as::<_, DokployApplication>(
            "SELECT a.\"applicationId\", a.name, a.\"sourceType\"::TEXT, a.repository, a.owner, a.branch, a.\"buildPath\", \
             a.\"customGitUrl\", a.\"customGitBranch\", a.\"buildType\"::TEXT, a.env, e.\"projectId\", \
             a.\"giteaRepository\", a.\"giteaOwner\" \
             FROM \"application\" a \
             JOIN \"environment\" e ON a.\"environmentId\" = e.\"environmentId\""
        )
        .fetch_all(&self.pool)
        .await?;

        let composes = sqlx::query_as::<_, DokployCompose>(
            "SELECT c.\"composeId\", c.name, c.\"composeFile\", e.\"projectId\", c.env \
             FROM \"compose\" c \
             JOIN \"environment\" e ON c.\"environmentId\" = e.\"environmentId\""
        )
        .fetch_all(&self.pool)
        .await?;

        let domains = sqlx::query_as::<_, DokployDomain>(
            "SELECT host, https, port, \"applicationId\", \"composeId\" FROM \"domain\""
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(DokployData {
            projects,
            applications,
            composes,
            domains,
        })
    }
}
