use reqwest::{Client, ClientBuilder};
use tracing::debug;

use super::error::ApiError;
use super::types::{
    AuthToken, ComponentsResponse, ProjectsResponse, VersionsResponse, VulnerabilitiesResponse,
};

#[derive(Debug, Clone)]
pub struct BlackDuckClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl BlackDuckClient {
    pub fn new(base_url: &str, accept_invalid_certs: bool) -> Result<Self, ApiError> {
        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(accept_invalid_certs)
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token: None,
        })
    }

    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// Authenticate with an API token and store the resulting bearer token.
    pub async fn authenticate(&mut self, api_token: &str) -> Result<(), ApiError> {
        let url = format!("{}/api/tokens/authenticate", self.base_url);
        debug!("Authenticating at {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {api_token}"))
            .header("Accept", "application/vnd.blackducksoftware.user-4+json")
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::StatusCode {
                context: "Authentication failed",
                status,
                body,
            });
        }

        let auth: AuthToken = response.json().await?;
        self.token = Some(auth.bearer_token);
        Ok(())
    }

    fn auth_header(&self) -> Result<String, ApiError> {
        self.token
            .as_ref()
            .map(|t| format!("Bearer {t}"))
            .ok_or(ApiError::NotAuthenticated)
    }

    /// Fetch a page of projects.
    pub async fn get_projects(
        &self,
        offset: u32,
        limit: u32,
    ) -> Result<ProjectsResponse, ApiError> {
        let url = format!(
            "{}/api/projects?offset={offset}&limit={limit}",
            self.base_url
        );
        debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header()?)
            .header(
                "Accept",
                "application/vnd.blackducksoftware.project-detail-4+json",
            )
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::StatusCode {
                context: "Failed to fetch projects",
                status,
                body,
            });
        }

        Ok(response.json::<ProjectsResponse>().await?)
    }

    /// Fetch a page of versions for a project.
    pub async fn get_versions(
        &self,
        project_href: &str,
        offset: u32,
        limit: u32,
    ) -> Result<VersionsResponse, ApiError> {
        let url = format!("{project_href}/versions?offset={offset}&limit={limit}");
        debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header()?)
            .header(
                "Accept",
                "application/vnd.blackducksoftware.project-detail-5+json",
            )
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::StatusCode {
                context: "Failed to fetch versions",
                status,
                body,
            });
        }

        Ok(response.json::<VersionsResponse>().await?)
    }

    /// Fetch a page of BOM components for a version.
    pub async fn get_components(
        &self,
        version_href: &str,
        offset: u32,
        limit: u32,
    ) -> Result<ComponentsResponse, ApiError> {
        let url = format!("{version_href}/components?offset={offset}&limit={limit}");
        debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header()?)
            .header(
                "Accept",
                "application/vnd.blackducksoftware.bill-of-materials-6+json",
            )
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::StatusCode {
                context: "Failed to fetch components",
                status,
                body,
            });
        }

        Ok(response.json::<ComponentsResponse>().await?)
    }

    /// Fetch a page of vulnerabilities for a version.
    pub async fn get_vulnerabilities(
        &self,
        version_href: &str,
        offset: u32,
        limit: u32,
    ) -> Result<VulnerabilitiesResponse, ApiError> {
        let url = format!("{version_href}/vulnerable-bom-components?offset={offset}&limit={limit}");
        debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header()?)
            .header(
                "Accept",
                "application/vnd.blackducksoftware.bill-of-materials-6+json",
            )
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::StatusCode {
                context: "Failed to fetch vulnerabilities",
                status,
                body,
            });
        }

        Ok(response.json::<VulnerabilitiesResponse>().await?)
    }

    /// Fetch BOM components that are in policy violation for a version.
    ///
    /// Uses `GET {version_href}/components?filter=policyStatus:IN_VIOLATION` which returns
    /// the same `ComponentsResponse` shape as `get_components`, but filtered to only the
    /// components whose `policyStatus` is `"IN_VIOLATION"`.
    pub async fn get_policy_violations(
        &self,
        version_href: &str,
        offset: u32,
        limit: u32,
    ) -> Result<ComponentsResponse, ApiError> {
        let url = format!(
            "{version_href}/components?filter=policyStatus:IN_VIOLATION&offset={offset}&limit={limit}"
        );
        debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header()?)
            .header(
                "Accept",
                "application/vnd.blackducksoftware.bill-of-materials-6+json",
            )
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::StatusCode {
                context: "Failed to fetch policy violations",
                status,
                body,
            });
        }

        Ok(response.json::<ComponentsResponse>().await?)
    }
}
