use reqwest::{Client, ClientBuilder};
use tracing::debug;

use super::error::ApiError;
use super::types::{
    AuthToken, ComponentFiltersResponse, ComponentsResponse, PolicyRulesResponse, ProjectsResponse,
    VersionsResponse, VulnerabilitiesResponse,
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
    ///
    /// `filter_params` is a slice of `("filter", "fieldName:VALUE")` pairs built
    /// from `ComponentFilter::to_api_params()`. Pass an empty slice for no filter.
    pub async fn get_components(
        &self,
        version_href: &str,
        offset: u32,
        limit: u32,
        filter_params: &[(&str, String)],
    ) -> Result<ComponentsResponse, ApiError> {
        let filter_qs: String = filter_params.iter().fold(String::new(), |mut acc, (k, v)| {
            use std::fmt::Write as _;
            let _ = write!(acc, "&{k}={v}");
            acc
        });
        let url = format!("{version_href}/components?offset={offset}&limit={limit}{filter_qs}");
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
    ///
    /// `extra_filter_params` allows additional `filter=` params (e.g. `reviewStatus:REVIEWED`)
    /// to be appended after the hardcoded `policyStatus:IN_VIOLATION` filter.
    pub async fn get_policy_violations(
        &self,
        version_href: &str,
        offset: u32,
        limit: u32,
        extra_filter_params: &[(&str, String)],
    ) -> Result<ComponentsResponse, ApiError> {
        let extra_qs: String = extra_filter_params
            .iter()
            .fold(String::new(), |mut acc, (k, v)| {
                use std::fmt::Write as _;
                let _ = write!(acc, "&{k}={v}");
                acc
            });
        let url = format!(
            "{version_href}/components?filter=policyStatus:IN_VIOLATION&offset={offset}&limit={limit}{extra_qs}"
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

    /// Fetch a page of violated policy rules for a specific BOM component.
    ///
    /// The `policy_rules_href` comes from the component's `_meta.links` where `rel == "policy-rules"`.
    /// Pass `offset = 0` and a suitable `limit` (e.g. 100) for the first page, then advance
    /// `offset` until `offset >= total_count` to paginate through all rules.
    ///
    /// NOTE: This method is kept for potential future use, but currently unused since we fetch
    /// policy rules via `/components-filters` endpoint instead.
    #[expect(dead_code, reason = "Kept for potential future use")]
    pub async fn get_component_policy_rules(
        &self,
        policy_rules_href: &str,
        offset: u32,
        limit: u32,
    ) -> Result<PolicyRulesResponse, ApiError> {
        let url = format!("{policy_rules_href}?offset={offset}&limit={limit}");
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
                context: "Failed to fetch component policy rules",
                status,
                body,
            });
        }

        Ok(response.json::<PolicyRulesResponse>().await?)
    }

    /// Fetch available filter options for a version's components.
    ///
    /// Example: `filterKey=policyRuleViolation` returns all policy rules that have violations
    /// in this version, with their IDs (already prefixed with "PR~") and display names.
    pub async fn get_component_filters(
        &self,
        version_href: &str,
        filter_key: &str,
    ) -> Result<ComponentFiltersResponse, ApiError> {
        let url = format!("{version_href}/components-filters?filterKey={filter_key}");
        debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header()?)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::StatusCode {
                context: "Failed to fetch component filters",
                status,
                body,
            });
        }

        Ok(response.json::<ComponentFiltersResponse>().await?)
    }
}
