use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    #[serde(rename = "bearerToken")]
    pub bearer_token: String,
    #[serde(rename = "expiresInMilliseconds")]
    pub expires_in_milliseconds: u64,
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Project {
    pub name: String,
    pub description: Option<String>,
    // Field name matches the Black Duck API JSON key; renaming would break deserialization.
    #[expect(
        clippy::struct_field_names,
        reason = "matches Black Duck API JSON field name"
    )]
    #[serde(rename = "projectLevelAdjustments")]
    pub project_level_adjustments: Option<bool>,
    #[serde(rename = "cloneCategories")]
    pub clone_categories: Option<Vec<String>>,
    #[serde(rename = "_meta")]
    pub meta: Option<Meta>,
}

impl Project {
    /// Returns the canonical href for this project from its `_meta` link.
    pub fn href(&self) -> Option<&str> {
        self.meta.as_ref().map(|m| m.href.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectsResponse {
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    pub items: Vec<Project>,
}

// ---------------------------------------------------------------------------
// Versions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectVersion {
    #[serde(rename = "versionName")]
    pub version_name: String,
    pub phase: Option<String>,
    pub distribution: Option<String>,
    #[serde(rename = "releaseComments")]
    pub release_comments: Option<String>,
    #[serde(rename = "releasedOn")]
    pub released_on: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "riskProfile")]
    pub risk_profile: Option<RiskProfile>,
    #[serde(rename = "_meta")]
    pub meta: Option<Meta>,
}

impl ProjectVersion {
    /// Returns the canonical href for this version from its `_meta` link.
    pub fn href(&self) -> Option<&str> {
        self.meta.as_ref().map(|m| m.href.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskProfile {
    pub categories: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionsResponse {
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    pub items: Vec<ProjectVersion>,
}

// ---------------------------------------------------------------------------
// Components (BOM)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BomComponent {
    #[serde(rename = "componentName")]
    pub component_name: String,
    #[serde(rename = "componentVersionName")]
    pub component_version_name: Option<String>,
    #[serde(rename = "ignored")]
    pub ignored: Option<bool>,
    #[serde(rename = "reviewStatus")]
    pub review_status: Option<String>,
    #[serde(rename = "approvalStatus")]
    pub approval_status: Option<String>,
    /// Overall policy status for this BOM entry (e.g. `"IN_VIOLATION"`, `"NOT_IN_VIOLATION"`).
    #[serde(rename = "policyStatus")]
    pub policy_status: Option<String>,
    pub usages: Option<Vec<String>>,
    pub licenses: Option<Vec<ComponentLicense>>,
    #[serde(rename = "securityRiskProfile")]
    pub security_risk_profile: Option<RiskCount>,
    #[serde(rename = "licenseRiskProfile")]
    pub license_risk_profile: Option<RiskCount>,
    #[serde(rename = "operationalRiskProfile")]
    pub operational_risk_profile: Option<RiskCount>,
    #[serde(rename = "_meta")]
    pub meta: Option<Meta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentLicense {
    #[serde(rename = "licenseType")]
    pub license_type: Option<String>,
    pub licenses: Option<Vec<LicenseInfo>>,
    #[serde(rename = "spdxId")]
    pub spdx_id: Option<String>,
    #[serde(rename = "licenseName")]
    pub license_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LicenseInfo {
    pub name: Option<String>,
    #[serde(rename = "spdxId")]
    pub spdx_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskCount {
    pub counts: Option<Vec<RiskCountEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskCountEntry {
    #[serde(rename = "countType")]
    pub count_type: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentsResponse {
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    pub items: Vec<BomComponent>,
}

// ---------------------------------------------------------------------------
// Vulnerabilities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Vulnerability {
    // Field name matches the Black Duck API JSON key; renaming would break deserialization.
    #[expect(
        clippy::struct_field_names,
        reason = "matches Black Duck API JSON field name"
    )]
    #[serde(rename = "vulnerabilityWithRemediation")]
    pub vulnerability_with_remediation: Option<VulnerabilityDetail>,
    #[serde(rename = "componentName")]
    pub component_name: Option<String>,
    #[serde(rename = "componentVersionName")]
    pub component_version_name: Option<String>,
    #[serde(rename = "_meta")]
    pub meta: Option<Meta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VulnerabilityDetail {
    #[serde(rename = "vulnerabilityName")]
    pub vulnerability_name: String,
    pub description: Option<String>,
    pub severity: Option<String>,
    #[serde(rename = "cvss2Severity")]
    pub cvss2_severity: Option<String>,
    #[serde(rename = "cvss3Severity")]
    pub cvss3_severity: Option<String>,
    #[serde(rename = "cvss2Score")]
    pub cvss2_score: Option<f64>,
    #[serde(rename = "cvss3Score")]
    pub cvss3_score: Option<f64>,
    #[serde(rename = "remediationStatus")]
    pub remediation_status: Option<String>,
    #[serde(rename = "remediationComment")]
    pub remediation_comment: Option<String>,
    #[serde(rename = "publishedDate")]
    pub published_date: Option<String>,
    #[serde(rename = "updatedDate")]
    pub updated_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilitiesResponse {
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    pub items: Vec<Vulnerability>,
}

// ---------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Meta {
    pub href: String,
    pub links: Option<Vec<Link>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Link {
    pub rel: String,
    pub href: String,
}
