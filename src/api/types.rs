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

impl BomComponent {
    /// Returns the href for this component's policy-rules link from `_meta.links`,
    /// or `None` if no such link exists.
    #[cfg_attr(not(test), expect(dead_code))]
    pub fn policy_rules_href(&self) -> Option<&str> {
        self.meta
            .as_ref()
            .and_then(|m| m.links.as_ref())
            .and_then(|links| {
                links
                    .iter()
                    .find(|l| l.rel == "policy-rules")
                    .map(|l| l.href.as_str())
            })
    }
}

// ---------------------------------------------------------------------------
// Component Filters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentFilterOption {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentFiltersResponse {
    pub name: String,
    pub label: String,
    pub values: Vec<ComponentFilterOption>,
    #[serde(rename = "_meta")]
    pub meta: Option<Meta>,
}

// ---------------------------------------------------------------------------
// Policy Rules
// ---------------------------------------------------------------------------

/// A policy rule that has been violated by a BOM component.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyRule {
    /// Human-readable name of the policy rule.
    #[serde(rename = "policyName")]
    pub policy_name: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "_meta")]
    pub meta: Option<Meta>,
}

impl PolicyRule {
    /// Returns the best display name for this rule: `policyName` if set, then `name`, else `""`.
    #[cfg_attr(not(test), expect(dead_code))]
    pub fn display_name(&self) -> &str {
        self.policy_name
            .as_deref()
            .or(self.name.as_deref())
            .unwrap_or("")
    }

    /// Extract the policy rule ID from the meta href.
    ///
    /// Example: `<https://host/api/policy-rules/abc-123>` → `"abc-123"`
    #[expect(dead_code)]
    pub fn id(&self) -> Option<&str> {
        self.meta.as_ref().and_then(|m| m.href.rsplit('/').next())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRulesResponse {
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    pub items: Vec<PolicyRule>,
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // AuthToken
    // ------------------------------------------------------------------

    #[test]
    fn auth_token_deserializes() {
        let json = r#"{"bearerToken":"abc123","expiresInMilliseconds":3600000}"#;
        let token: AuthToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.bearer_token, "abc123");
        assert_eq!(token.expires_in_milliseconds, 3_600_000);
    }

    // ------------------------------------------------------------------
    // Project
    // ------------------------------------------------------------------

    #[test]
    fn project_href_returns_meta_href() {
        let p = Project {
            name: "MyProject".to_string(),
            meta: Some(Meta {
                href: "https://bd.example.com/api/projects/1".to_string(),
                links: None,
            }),
            ..Project::default()
        };
        assert_eq!(p.href(), Some("https://bd.example.com/api/projects/1"));
    }

    #[test]
    fn project_href_none_when_no_meta() {
        let p = Project::default();
        assert_eq!(p.href(), None);
    }

    #[test]
    fn project_deserializes_with_description() {
        let json = r#"{
            "name": "Test",
            "description": "A project",
            "_meta": {"href": "https://example.com/projects/1"}
        }"#;
        let p: Project = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "Test");
        assert_eq!(p.description.as_deref(), Some("A project"));
        assert_eq!(p.href(), Some("https://example.com/projects/1"));
    }

    #[test]
    fn projects_response_deserializes() {
        let json = r#"{"totalCount":1,"items":[{"name":"P1"}]}"#;
        let resp: ProjectsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.total_count, 1);
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].name, "P1");
    }

    // ------------------------------------------------------------------
    // ProjectVersion
    // ------------------------------------------------------------------

    #[test]
    fn project_version_href_returns_meta_href() {
        let v = ProjectVersion {
            version_name: "1.0".to_string(),
            meta: Some(Meta {
                href: "https://bd.example.com/api/projects/1/versions/2".to_string(),
                links: None,
            }),
            ..ProjectVersion::default()
        };
        assert_eq!(
            v.href(),
            Some("https://bd.example.com/api/projects/1/versions/2")
        );
    }

    #[test]
    fn project_version_href_none_without_meta() {
        assert_eq!(ProjectVersion::default().href(), None);
    }

    #[test]
    fn project_version_deserializes() {
        let json = r#"{
            "versionName": "2.0",
            "phase": "RELEASED",
            "distribution": "EXTERNAL"
        }"#;
        let v: ProjectVersion = serde_json::from_str(json).unwrap();
        assert_eq!(v.version_name, "2.0");
        assert_eq!(v.phase.as_deref(), Some("RELEASED"));
        assert_eq!(v.distribution.as_deref(), Some("EXTERNAL"));
    }

    // ------------------------------------------------------------------
    // BomComponent
    // ------------------------------------------------------------------

    #[test]
    fn bom_component_deserializes_all_status_fields() {
        let json = r#"{
            "componentName": "log4j",
            "componentVersionName": "2.14.1",
            "reviewStatus": "REVIEWED",
            "approvalStatus": "APPROVED",
            "policyStatus": "IN_VIOLATION"
        }"#;
        let c: BomComponent = serde_json::from_str(json).unwrap();
        assert_eq!(c.component_name, "log4j");
        assert_eq!(c.component_version_name.as_deref(), Some("2.14.1"));
        assert_eq!(c.review_status.as_deref(), Some("REVIEWED"));
        assert_eq!(c.approval_status.as_deref(), Some("APPROVED"));
        assert_eq!(c.policy_status.as_deref(), Some("IN_VIOLATION"));
    }

    #[test]
    fn bom_component_optional_fields_absent() {
        let json = r#"{"componentName":"minimal"}"#;
        let c: BomComponent = serde_json::from_str(json).unwrap();
        assert_eq!(c.component_name, "minimal");
        assert!(c.component_version_name.is_none());
        assert!(c.review_status.is_none());
        assert!(c.approval_status.is_none());
        assert!(c.policy_status.is_none());
        assert!(c.licenses.is_none());
    }

    #[test]
    fn components_response_deserializes() {
        let json = r#"{"totalCount":2,"items":[{"componentName":"A"},{"componentName":"B"}]}"#;
        let resp: ComponentsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.total_count, 2);
        assert_eq!(resp.items.len(), 2);
    }

    // ------------------------------------------------------------------
    // Vulnerability / VulnerabilityDetail
    // ------------------------------------------------------------------

    #[test]
    fn vulnerability_deserializes_with_detail() {
        let json = r#"{
            "componentName": "openssl",
            "componentVersionName": "1.0.1",
            "vulnerabilityWithRemediation": {
                "vulnerabilityName": "CVE-2014-0160",
                "severity": "CRITICAL",
                "cvss3Score": 9.8,
                "cvss2Score": 7.5,
                "remediationStatus": "NEEDS_REVIEW"
            }
        }"#;
        let v: Vulnerability = serde_json::from_str(json).unwrap();
        assert_eq!(v.component_name.as_deref(), Some("openssl"));
        let detail = v.vulnerability_with_remediation.unwrap();
        assert_eq!(detail.vulnerability_name, "CVE-2014-0160");
        assert_eq!(detail.severity.as_deref(), Some("CRITICAL"));
        assert!((detail.cvss3_score.unwrap() - 9.8).abs() < f64::EPSILON);
        assert!((detail.cvss2_score.unwrap() - 7.5).abs() < f64::EPSILON);
    }

    #[test]
    fn vulnerability_detail_missing_scores_are_none() {
        let json = r#"{"vulnerabilityName":"CVE-X"}"#;
        let d: VulnerabilityDetail = serde_json::from_str(json).unwrap();
        assert!(d.cvss3_score.is_none());
        assert!(d.cvss2_score.is_none());
    }

    #[test]
    fn vulnerabilities_response_deserializes() {
        let json = r#"{"totalCount":0,"items":[]}"#;
        let resp: VulnerabilitiesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.total_count, 0);
        assert!(resp.items.is_empty());
    }

    // ------------------------------------------------------------------
    // RiskCount / RiskCountEntry
    // ------------------------------------------------------------------

    #[test]
    fn risk_count_deserializes() {
        let json = r#"{"counts":[{"countType":"HIGH","count":3},{"countType":"LOW","count":0}]}"#;
        let r: RiskCount = serde_json::from_str(json).unwrap();
        let counts = r.counts.unwrap();
        assert_eq!(counts.len(), 2);
        assert_eq!(counts[0].count_type, "HIGH");
        assert_eq!(counts[0].count, 3);
    }

    // ------------------------------------------------------------------
    // ComponentLicense / LicenseInfo
    // ------------------------------------------------------------------

    #[test]
    fn component_license_deserializes_license_name() {
        let json = r#"{"licenseName":"MIT","spdxId":"MIT"}"#;
        let lic: ComponentLicense = serde_json::from_str(json).unwrap();
        assert_eq!(lic.license_name.as_deref(), Some("MIT"));
        assert_eq!(lic.spdx_id.as_deref(), Some("MIT"));
    }

    #[test]
    fn license_info_deserializes() {
        let json = r#"{"name":"Apache-2.0","spdxId":"Apache-2.0"}"#;
        let li: LicenseInfo = serde_json::from_str(json).unwrap();
        assert_eq!(li.name.as_deref(), Some("Apache-2.0"));
    }

    // ------------------------------------------------------------------
    // Meta / Link
    // ------------------------------------------------------------------

    #[test]
    fn meta_deserializes_with_links() {
        let json = r#"{
            "href": "https://example.com/api/resource",
            "links": [{"rel":"versions","href":"https://example.com/api/resource/versions"}]
        }"#;
        let m: Meta = serde_json::from_str(json).unwrap();
        assert_eq!(m.href, "https://example.com/api/resource");
        let links = m.links.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].rel, "versions");
    }

    // ------------------------------------------------------------------
    // BomComponent::policy_rules_href
    // ------------------------------------------------------------------

    #[test]
    fn bom_component_policy_rules_href_found() {
        let c = BomComponent {
            component_name: "lib".to_string(),
            meta: Some(Meta {
                href: "https://example.com/api/projects/1/versions/2/components/3".to_string(),
                links: Some(vec![
                    Link {
                        rel: "policy-rules".to_string(),
                        href: "https://example.com/api/projects/1/versions/2/components/3/policy-rules"
                            .to_string(),
                    },
                    Link {
                        rel: "other".to_string(),
                        href: "https://example.com/other".to_string(),
                    },
                ]),
            }),
            ..BomComponent::default()
        };
        assert_eq!(
            c.policy_rules_href(),
            Some("https://example.com/api/projects/1/versions/2/components/3/policy-rules")
        );
    }

    #[test]
    fn bom_component_policy_rules_href_none_when_no_link() {
        let c = BomComponent {
            component_name: "lib".to_string(),
            meta: Some(Meta {
                href: "https://example.com/api/component".to_string(),
                links: Some(vec![Link {
                    rel: "other".to_string(),
                    href: "https://example.com/other".to_string(),
                }]),
            }),
            ..BomComponent::default()
        };
        assert_eq!(c.policy_rules_href(), None);
    }

    #[test]
    fn bom_component_policy_rules_href_none_when_no_meta() {
        let c = BomComponent::default();
        assert_eq!(c.policy_rules_href(), None);
    }

    // ------------------------------------------------------------------
    // PolicyRule::display_name
    // ------------------------------------------------------------------

    #[test]
    fn policy_rule_display_name_prefers_policy_name() {
        let r = PolicyRule {
            policy_name: Some("Copyleft Licenses".to_string()),
            name: Some("other".to_string()),
            ..PolicyRule::default()
        };
        assert_eq!(r.display_name(), "Copyleft Licenses");
    }

    #[test]
    fn policy_rule_display_name_falls_back_to_name() {
        let r = PolicyRule {
            policy_name: None,
            name: Some("Security Policy".to_string()),
            ..PolicyRule::default()
        };
        assert_eq!(r.display_name(), "Security Policy");
    }

    #[test]
    fn policy_rule_display_name_empty_when_both_none() {
        let r = PolicyRule::default();
        assert_eq!(r.display_name(), "");
    }

    #[test]
    fn policy_rules_response_deserializes() {
        let json = r#"{
            "totalCount": 1,
            "items": [{"policyName": "Copyleft Licenses", "name": "cl-rule"}]
        }"#;
        let resp: PolicyRulesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.total_count, 1);
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].display_name(), "Copyleft Licenses");
    }
}
