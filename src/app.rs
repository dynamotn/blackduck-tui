use crate::api::{BomComponent, Project, ProjectVersion, Vulnerability};
use crate::config::Config;

/// Which panel is currently focused
#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Left,
    Right,
}

/// The current "screen" / navigation level
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Login,
    Projects,
    Versions,
    Components,
    Vulnerabilities,
    PolicyViolations,
}

/// Right panel tab selection (when inside a version)
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum VersionTab {
    Components,
    Vulnerabilities,
    PolicyViolations,
}

impl VersionTab {
    pub fn next(self) -> Self {
        match self {
            Self::Components => Self::Vulnerabilities,
            Self::Vulnerabilities => Self::PolicyViolations,
            Self::PolicyViolations => Self::Components,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Components => Self::PolicyViolations,
            Self::Vulnerabilities => Self::Components,
            Self::PolicyViolations => Self::Vulnerabilities,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Components => "Components",
            Self::Vulnerabilities => "Vulnerabilities",
            Self::PolicyViolations => "Policy Violations",
        }
    }
}

// ---------------------------------------------------------------------------
// Filter state
// ---------------------------------------------------------------------------

/// Which field is being edited in the filter popup
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum FilterField {
    PolicyStatus,
    ReviewStatus,
    ApprovalStatus,
    PolicyRuleName,
}

impl FilterField {
    pub const ALL: [Self; 4] = [
        Self::PolicyStatus,
        Self::ReviewStatus,
        Self::ApprovalStatus,
        Self::PolicyRuleName,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::PolicyStatus => "Policy Status",
            Self::ReviewStatus => "Review Status",
            Self::ApprovalStatus => "Approval Status",
            Self::PolicyRuleName => "Policy Rule Name",
        }
    }

    /// Known static option values for each filter field.
    /// `PolicyRuleName` returns an empty slice — options are populated dynamically from the API.
    pub fn options(self) -> &'static [&'static str] {
        match self {
            Self::PolicyStatus => &[
                "IN_VIOLATION",
                "IN_VIOLATION_OVERRIDDEN",
                "NOT_IN_VIOLATION",
            ],
            Self::ReviewStatus => &["UNREVIEWED", "REVIEWED", "DYNAMIC", "MANUAL"],
            Self::ApprovalStatus => &["APPROVED", "REJECTED", "PENDING"],
            Self::PolicyRuleName => &[],
        }
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "used by filter popup UI (not yet wired)")
    )]
    pub fn next(self) -> Self {
        match self {
            Self::PolicyStatus => Self::ReviewStatus,
            Self::ReviewStatus => Self::ApprovalStatus,
            Self::ApprovalStatus => Self::PolicyRuleName,
            Self::PolicyRuleName => Self::PolicyStatus,
        }
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "used by filter popup UI (not yet wired)")
    )]
    pub fn prev(self) -> Self {
        match self {
            Self::PolicyStatus => Self::PolicyRuleName,
            Self::ReviewStatus => Self::PolicyStatus,
            Self::ApprovalStatus => Self::ReviewStatus,
            Self::PolicyRuleName => Self::ApprovalStatus,
        }
    }
}

/// Active filter values applied to component lists
#[derive(Debug, Clone, Default)]
pub struct ComponentFilter {
    /// If non-empty, only components whose `policy_status` is in this set are shown.
    pub policy_statuses: Vec<String>,
    /// If non-empty, only components whose `review_status` is in this set are shown.
    pub review_statuses: Vec<String>,
    /// If non-empty, only components whose `approval_status` is in this set are shown.
    pub approval_statuses: Vec<String>,
    /// If non-empty, filter by policy rule IDs (for server-side filtering).
    /// This is sent to the API as `filter=policyRuleViolation:ID`.
    pub rule_ids: Vec<String>,
}

impl ComponentFilter {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "used by filter popup UI (not yet wired)")
    )]
    pub fn is_empty(&self) -> bool {
        self.policy_statuses.is_empty()
            && self.review_statuses.is_empty()
            && self.approval_statuses.is_empty()
            && self.rule_ids.is_empty()
    }

    /// Number of active filter criteria (for badge display).
    pub fn active_count(&self) -> usize {
        usize::from(!self.policy_statuses.is_empty())
            + usize::from(!self.review_statuses.is_empty())
            + usize::from(!self.approval_statuses.is_empty())
            + usize::from(!self.rule_ids.is_empty())
    }

    /// Toggle a value in a filter set (add if absent, remove if present).
    pub fn toggle(set: &mut Vec<String>, value: &str) {
        if let Some(pos) = set.iter().position(|v| v == value) {
            set.remove(pos);
        } else {
            set.push(value.to_string());
        }
    }

    /// Build Black Duck API `filter=` query params from the active filter state.
    ///
    /// Returns a list of `("filter", "fieldName:VALUE")` pairs suitable for
    /// appending to a request URL, including policy rule violation IDs.
    pub fn to_api_params(&self) -> Vec<(&'static str, String)> {
        let mut params: Vec<(&'static str, String)> = Vec::new();
        for s in &self.policy_statuses {
            params.push(("filter", format!("policyStatus:{s}")));
        }
        for s in &self.review_statuses {
            params.push(("filter", format!("reviewStatus:{s}")));
        }
        for s in &self.approval_statuses {
            params.push(("filter", format!("approvalStatus:{s}")));
        }
        for id in &self.rule_ids {
            // The ID already has "PR~" prefix from the /components-filters API
            params.push(("filter", format!("policyRuleViolation:{id}")));
        }
        params
    }

    /// Returns true if `component` passes all active filters.
    /// Note: `rule_ids` filtering is done server-side via API, so not checked here.
    pub fn matches(&self, c: &BomComponent) -> bool {
        if !self.policy_statuses.is_empty() {
            let status = c.policy_status.as_deref().unwrap_or("");
            if !self.policy_statuses.iter().any(|s| s == status) {
                return false;
            }
        }
        if !self.review_statuses.is_empty() {
            let status = c.review_status.as_deref().unwrap_or("");
            if !self.review_statuses.iter().any(|s| s == status) {
                return false;
            }
        }
        if !self.approval_statuses.is_empty() {
            let status = c.approval_status.as_deref().unwrap_or("");
            if !self.approval_statuses.iter().any(|s| s == status) {
                return false;
            }
        }
        true
    }
}

/// State for the filter popup overlay
#[derive(Debug, Clone, Default)]
pub struct FilterPopup {
    pub open: bool,
    /// Which filter field row is currently highlighted
    pub focused_field: usize,
    /// Within the focused field, which option row is highlighted
    pub focused_option: usize,
}

impl FilterPopup {
    pub fn current_field(&self) -> FilterField {
        FilterField::ALL[self.focused_field % FilterField::ALL.len()]
    }

    pub fn move_field_down(&mut self) {
        self.focused_field = (self.focused_field + 1) % FilterField::ALL.len();
        self.focused_option = 0;
    }

    pub fn move_field_up(&mut self) {
        self.focused_field =
            (self.focused_field + FilterField::ALL.len() - 1) % FilterField::ALL.len();
        self.focused_option = 0;
    }

    pub fn move_option_down(&mut self, options_count: usize) {
        if options_count == 0 {
            return;
        }
        self.focused_option = (self.focused_option + 1) % options_count;
    }

    pub fn move_option_up(&mut self, options_count: usize) {
        if options_count == 0 {
            return;
        }
        self.focused_option = (self.focused_option + options_count - 1) % options_count;
    }
}

// ---------------------------------------------------------------------------
// AppEvent
// ---------------------------------------------------------------------------

/// Async messages sent from background tasks to the main loop
#[derive(Debug)]
pub enum AppEvent {
    ProjectsLoaded {
        items: Vec<Project>,
        total: u64,
        offset: u32,
    },
    VersionsLoaded {
        items: Vec<ProjectVersion>,
        total: u64,
        offset: u32,
    },
    ComponentsLoaded {
        items: Vec<BomComponent>,
        total: u64,
        offset: u32,
    },
    VulnerabilitiesLoaded {
        items: Vec<Vulnerability>,
        total: u64,
        offset: u32,
    },
    PolicyViolationsLoaded {
        items: Vec<BomComponent>,
        total: u64,
        offset: u32,
    },
    /// Policy rules fetched for all in-violation components of the current version.
    /// Each tuple contains (`display_name`, `rule_id`).
    PolicyRulesLoaded(Vec<(String, String)>),
    AuthSuccess,
    Error(String),
}

// ---------------------------------------------------------------------------
// StatefulList
// ---------------------------------------------------------------------------

/// Stateful list helper
#[derive(Debug, Default)]
pub struct StatefulList<T> {
    pub items: Vec<T>,
    pub selected: usize,
}

impl<T> StatefulList<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items, selected: 0 }
    }

    pub fn selected_item(&self) -> Option<&T> {
        self.items.get(self.selected)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

/// Main application state
pub struct App {
    pub config: Config,
    pub screen: Screen,
    pub focus: Focus,

    // Login form
    pub login_url_input: String,
    pub login_token_input: String,
    pub login_active_field: usize, // 0=url, 1=token
    pub login_error: Option<String>,

    // Data lists
    pub projects: StatefulList<Project>,
    pub projects_total: u64,
    pub projects_offset: u32,
    pub versions: StatefulList<ProjectVersion>,
    pub versions_total: u64,
    pub versions_offset: u32,
    pub components: StatefulList<BomComponent>,
    pub components_total: u64,
    pub components_offset: u32,
    pub vulnerabilities: StatefulList<Vulnerability>,
    pub vulnerabilities_total: u64,
    pub vulnerabilities_offset: u32,
    pub policy_violations: StatefulList<BomComponent>,
    pub policy_violations_total: u64,
    pub policy_violations_offset: u32,

    // Currently selected parent context
    pub selected_project: Option<Project>,
    pub selected_version: Option<ProjectVersion>,

    // Active tab in version detail view
    pub version_tab: VersionTab,

    // Loading / status
    pub loading: bool,
    pub status_message: Option<String>,
    pub error_message: Option<String>,

    // Quit flag
    pub should_quit: bool,

    // Search / filter
    pub search_input: String,
    pub search_active: bool,

    // Filter popup
    pub filter: ComponentFilter,
    pub filter_popup: FilterPopup,
    /// Deduplicated list of violated policy rules available for filtering.
    /// Each tuple contains (`display_name`, `rule_id`).
    /// Populated when the filter popup is opened (fetched from API on demand).
    pub available_policy_rules: Vec<(String, String)>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let has_url = !config.server.url.is_empty();
        let has_token = config.server.api_token.is_some();

        let screen = if has_url && has_token {
            Screen::Projects
        } else {
            Screen::Login
        };

        let login_url_input = config.server.url.clone();
        let login_token_input = config.server.api_token.clone().unwrap_or_default();

        Self {
            config,
            screen,
            focus: Focus::Left,

            login_url_input,
            login_token_input,
            login_active_field: 0,
            login_error: None,

            projects: StatefulList::default(),
            projects_total: 0,
            projects_offset: 0,
            versions: StatefulList::default(),
            versions_total: 0,
            versions_offset: 0,
            components: StatefulList::default(),
            components_total: 0,
            components_offset: 0,
            vulnerabilities: StatefulList::default(),
            vulnerabilities_total: 0,
            vulnerabilities_offset: 0,
            policy_violations: StatefulList::default(),
            policy_violations_total: 0,
            policy_violations_offset: 0,

            selected_project: None,
            selected_version: None,

            version_tab: VersionTab::Components,

            loading: false,
            status_message: None,
            error_message: None,

            should_quit: false,

            search_input: String::new(),
            search_active: false,

            filter: ComponentFilter::default(),
            filter_popup: FilterPopup::default(),
            available_policy_rules: Vec::new(),
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.error_message = None;
    }

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error_message = Some(msg.into());
        self.status_message = None;
        self.loading = false;
    }

    pub fn clear_messages(&mut self) {
        self.status_message = None;
        self.error_message = None;
    }

    /// Navigate back one level; also clears active filters.
    pub fn go_back(&mut self) {
        match self.screen {
            Screen::Login | Screen::Projects => {}
            Screen::Versions => {
                self.screen = Screen::Projects;
                self.selected_project = None;
                self.versions = StatefulList::default();
                self.versions_total = 0;
                self.versions_offset = 0;
                self.focus = Focus::Left;
            }
            Screen::Components | Screen::Vulnerabilities | Screen::PolicyViolations => {
                self.screen = Screen::Versions;
                self.selected_version = None;
                self.components = StatefulList::default();
                self.components_total = 0;
                self.components_offset = 0;
                self.vulnerabilities = StatefulList::default();
                self.vulnerabilities_total = 0;
                self.vulnerabilities_offset = 0;
                self.policy_violations = StatefulList::default();
                self.policy_violations_total = 0;
                self.policy_violations_offset = 0;
                self.filter = ComponentFilter::default();
                self.filter_popup = FilterPopup::default();
                self.available_policy_rules.clear();
                self.focus = Focus::Left;
            }
        }
        self.clear_messages();
    }

    // -----------------------------------------------------------------------
    // Selection clamping
    // -----------------------------------------------------------------------

    /// After any filter change, snap `components.selected` and
    /// `policy_violations.selected` to the first visible filtered index so
    /// that navigation (`j`/`k`) and the detail panel remain coherent.
    ///
    /// If the currently-selected raw index is still visible in the filtered
    /// view the selection is kept as-is. If it has been hidden the cursor
    /// jumps to the first visible item (index 0 of the filtered slice), or
    /// stays at 0 when the filtered view is empty.
    pub fn clamp_selection_to_filter(&mut self) {
        // Components
        let comp_visible: Vec<usize> = self
            .filtered_components()
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        if !comp_visible.contains(&self.components.selected) {
            self.components.selected = comp_visible.first().copied().unwrap_or(0);
        }

        // Policy violations
        let pv_visible: Vec<usize> = self
            .filtered_policy_violations()
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        if !pv_visible.contains(&self.policy_violations.selected) {
            self.policy_violations.selected = pv_visible.first().copied().unwrap_or(0);
        }
    }

    // -----------------------------------------------------------------------
    // Filtered views
    // -----------------------------------------------------------------------

    pub fn filtered_projects(&self) -> Vec<(usize, &Project)> {
        let q = self.search_input.to_lowercase();
        self.projects
            .items
            .iter()
            .enumerate()
            .filter(|(_, p)| q.is_empty() || p.name.to_lowercase().contains(&q))
            .collect()
    }

    pub fn filtered_versions(&self) -> Vec<(usize, &ProjectVersion)> {
        let q = self.search_input.to_lowercase();
        self.versions
            .items
            .iter()
            .enumerate()
            .filter(|(_, v)| q.is_empty() || v.version_name.to_lowercase().contains(&q))
            .collect()
    }

    pub fn filtered_components(&self) -> Vec<(usize, &BomComponent)> {
        let q = self.search_input.to_lowercase();
        self.components
            .items
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                // text search
                let text_ok = q.is_empty()
                    || c.component_name.to_lowercase().contains(&q)
                    || c.component_version_name
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q);
                text_ok && self.filter.matches(c)
            })
            .collect()
    }

    pub fn filtered_vulnerabilities(&self) -> Vec<(usize, &Vulnerability)> {
        let q = self.search_input.to_lowercase();
        self.vulnerabilities
            .items
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                if q.is_empty() {
                    return true;
                }
                let name = v
                    .vulnerability_with_remediation
                    .as_ref()
                    .map(|d| d.vulnerability_name.to_lowercase())
                    .unwrap_or_default();
                let comp = v.component_name.as_deref().unwrap_or("").to_lowercase();
                name.contains(&q) || comp.contains(&q)
            })
            .collect()
    }

    pub fn filtered_policy_violations(&self) -> Vec<(usize, &BomComponent)> {
        let q = self.search_input.to_lowercase();
        self.policy_violations
            .items
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                let text_ok = q.is_empty()
                    || c.component_name.to_lowercase().contains(&q)
                    || c.component_version_name
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q);
                // For policy violations, only apply review/approval filters
                // (policy_status filter doesn't make sense here since all are IN_VIOLATION by API)
                let filter_ok = {
                    let mut f = self.filter.clone();
                    f.policy_statuses.clear(); // ignore policy_status filter on this tab
                    f.matches(c)
                };
                text_ok && filter_ok
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::BomComponent;

    // Helper: create a minimal BomComponent with given status fields
    fn make_component(
        name: &str,
        policy: Option<&str>,
        review: Option<&str>,
        approval: Option<&str>,
    ) -> BomComponent {
        BomComponent {
            component_name: name.to_string(),
            policy_status: policy.map(ToString::to_string),
            review_status: review.map(ToString::to_string),
            approval_status: approval.map(ToString::to_string),
            ..BomComponent::default()
        }
    }

    // ------------------------------------------------------------------
    // FilterField
    // ------------------------------------------------------------------

    #[test]
    fn filter_field_label() {
        assert_eq!(FilterField::PolicyStatus.label(), "Policy Status");
        assert_eq!(FilterField::ReviewStatus.label(), "Review Status");
        assert_eq!(FilterField::ApprovalStatus.label(), "Approval Status");
        assert_eq!(FilterField::PolicyRuleName.label(), "Policy Rule Name");
    }

    #[test]
    fn filter_field_options_non_empty() {
        // PolicyRuleName has dynamic options — static slice is intentionally empty
        for field in FilterField::ALL {
            if field == FilterField::PolicyRuleName {
                assert!(
                    field.options().is_empty(),
                    "PolicyRuleName static options should be empty (dynamic)"
                );
            } else {
                assert!(!field.options().is_empty(), "{field:?} should have options");
            }
        }
    }

    #[test]
    fn filter_field_next_wraps() {
        assert_eq!(FilterField::PolicyStatus.next(), FilterField::ReviewStatus);
        assert_eq!(
            FilterField::ReviewStatus.next(),
            FilterField::ApprovalStatus
        );
        assert_eq!(
            FilterField::ApprovalStatus.next(),
            FilterField::PolicyRuleName
        );
        assert_eq!(
            FilterField::PolicyRuleName.next(),
            FilterField::PolicyStatus
        );
    }

    #[test]
    fn filter_field_prev_wraps() {
        assert_eq!(
            FilterField::PolicyStatus.prev(),
            FilterField::PolicyRuleName
        );
        assert_eq!(FilterField::ReviewStatus.prev(), FilterField::PolicyStatus);
        assert_eq!(
            FilterField::ApprovalStatus.prev(),
            FilterField::ReviewStatus
        );
        assert_eq!(
            FilterField::PolicyRuleName.prev(),
            FilterField::ApprovalStatus
        );
    }

    #[test]
    fn filter_field_all_covers_all_variants() {
        assert_eq!(FilterField::ALL.len(), 4);
        assert!(FilterField::ALL.contains(&FilterField::PolicyStatus));
        assert!(FilterField::ALL.contains(&FilterField::ReviewStatus));
        assert!(FilterField::ALL.contains(&FilterField::ApprovalStatus));
        assert!(FilterField::ALL.contains(&FilterField::PolicyRuleName));
    }

    // ------------------------------------------------------------------
    // ComponentFilter — is_empty / active_count
    // ------------------------------------------------------------------

    #[test]
    fn component_filter_default_is_empty() {
        let f = ComponentFilter::default();
        assert!(f.is_empty());
        assert_eq!(f.active_count(), 0);
    }

    #[test]
    fn component_filter_active_count_increments_per_field() {
        let mut f = ComponentFilter::default();
        f.policy_statuses.push("IN_VIOLATION".to_string());
        assert_eq!(f.active_count(), 1);
        assert!(!f.is_empty());

        f.review_statuses.push("REVIEWED".to_string());
        assert_eq!(f.active_count(), 2);

        f.approval_statuses.push("APPROVED".to_string());
        assert_eq!(f.active_count(), 3);

        f.rule_ids.push("rule-id-123".to_string());
        assert_eq!(f.active_count(), 4);
    }

    #[test]
    fn component_filter_active_count_is_per_field_not_per_value() {
        // Adding two values to the same field still counts as 1
        let mut f = ComponentFilter::default();
        f.policy_statuses.push("IN_VIOLATION".to_string());
        f.policy_statuses.push("NOT_IN_VIOLATION".to_string());
        assert_eq!(f.active_count(), 1);
    }

    // ------------------------------------------------------------------
    // ComponentFilter::toggle
    // ------------------------------------------------------------------

    #[test]
    fn toggle_adds_when_absent() {
        let mut set: Vec<String> = vec![];
        ComponentFilter::toggle(&mut set, "IN_VIOLATION");
        assert_eq!(set, vec!["IN_VIOLATION"]);
    }

    #[test]
    fn toggle_removes_when_present() {
        let mut set = vec!["IN_VIOLATION".to_string()];
        ComponentFilter::toggle(&mut set, "IN_VIOLATION");
        assert!(set.is_empty());
    }

    #[test]
    fn toggle_idempotent_add_then_remove() {
        let mut set: Vec<String> = vec![];
        ComponentFilter::toggle(&mut set, "REVIEWED");
        ComponentFilter::toggle(&mut set, "REVIEWED");
        assert!(set.is_empty());
    }

    #[test]
    fn toggle_only_removes_matching_value() {
        let mut set = vec!["REVIEWED".to_string(), "DYNAMIC".to_string()];
        ComponentFilter::toggle(&mut set, "REVIEWED");
        assert_eq!(set, vec!["DYNAMIC"]);
    }

    // ------------------------------------------------------------------
    // ComponentFilter::matches
    // ------------------------------------------------------------------

    #[test]
    fn matches_empty_filter_passes_everything() {
        let f = ComponentFilter::default();
        let c = make_component(
            "lib",
            Some("IN_VIOLATION"),
            Some("REVIEWED"),
            Some("APPROVED"),
        );
        assert!(f.matches(&c));
    }

    #[test]
    fn matches_policy_status_included() {
        let mut f = ComponentFilter::default();
        f.policy_statuses.push("IN_VIOLATION".to_string());

        let yes = make_component("a", Some("IN_VIOLATION"), None, None);
        let no = make_component("b", Some("NOT_IN_VIOLATION"), None, None);
        let missing = make_component("c", None, None, None);

        assert!(f.matches(&yes));
        assert!(!f.matches(&no));
        assert!(!f.matches(&missing)); // "" does not match
    }

    #[test]
    fn matches_policy_status_multiple_values_or_semantics() {
        let mut f = ComponentFilter::default();
        f.policy_statuses.push("IN_VIOLATION".to_string());
        f.policy_statuses
            .push("IN_VIOLATION_OVERRIDDEN".to_string());

        assert!(f.matches(&make_component("a", Some("IN_VIOLATION"), None, None)));
        assert!(f.matches(&make_component(
            "b",
            Some("IN_VIOLATION_OVERRIDDEN"),
            None,
            None
        )));
        assert!(!f.matches(&make_component("c", Some("NOT_IN_VIOLATION"), None, None)));
    }

    #[test]
    fn matches_review_status_filter() {
        let mut f = ComponentFilter::default();
        f.review_statuses.push("REVIEWED".to_string());

        assert!(f.matches(&make_component("a", None, Some("REVIEWED"), None)));
        assert!(!f.matches(&make_component("b", None, Some("UNREVIEWED"), None)));
    }

    #[test]
    fn matches_approval_status_filter() {
        let mut f = ComponentFilter::default();
        f.approval_statuses.push("APPROVED".to_string());

        assert!(f.matches(&make_component("a", None, None, Some("APPROVED"))));
        assert!(!f.matches(&make_component("b", None, None, Some("REJECTED"))));
    }

    #[test]
    fn matches_all_filters_must_pass_and_semantics() {
        let mut f = ComponentFilter::default();
        f.review_statuses.push("REVIEWED".to_string());
        f.approval_statuses.push("APPROVED".to_string());

        // Both match
        assert!(f.matches(&make_component(
            "a",
            None,
            Some("REVIEWED"),
            Some("APPROVED")
        )));
        // review OK, approval not
        assert!(!f.matches(&make_component(
            "b",
            None,
            Some("REVIEWED"),
            Some("PENDING")
        )));
        // approval OK, review not
        assert!(!f.matches(&make_component(
            "c",
            None,
            Some("UNREVIEWED"),
            Some("APPROVED")
        )));
    }

    #[test]
    fn filter_popup_move_option_noop_when_zero_options() {
        let mut p = FilterPopup {
            focused_option: 2,
            ..FilterPopup::default()
        };
        p.move_option_down(0); // should not panic or change state
        assert_eq!(p.focused_option, 2);
        p.move_option_up(0);
        assert_eq!(p.focused_option, 2);
    }

    // ------------------------------------------------------------------
    // FilterPopup navigation
    // ------------------------------------------------------------------

    #[test]
    fn filter_popup_current_field_default_is_policy_status() {
        let p = FilterPopup::default();
        assert_eq!(p.current_field(), FilterField::PolicyStatus);
    }

    #[test]
    fn filter_popup_move_field_down_cycles() {
        let mut p = FilterPopup::default();
        p.move_field_down();
        assert_eq!(p.current_field(), FilterField::ReviewStatus);
        p.move_field_down();
        assert_eq!(p.current_field(), FilterField::ApprovalStatus);
        p.move_field_down();
        assert_eq!(p.current_field(), FilterField::PolicyRuleName);
        p.move_field_down();
        assert_eq!(p.current_field(), FilterField::PolicyStatus);
    }

    #[test]
    fn filter_popup_move_field_up_cycles() {
        let mut p = FilterPopup::default();
        p.move_field_up();
        assert_eq!(p.current_field(), FilterField::PolicyRuleName);
        p.move_field_up();
        assert_eq!(p.current_field(), FilterField::ApprovalStatus);
        p.move_field_up();
        assert_eq!(p.current_field(), FilterField::ReviewStatus);
        p.move_field_up();
        assert_eq!(p.current_field(), FilterField::PolicyStatus);
    }

    #[test]
    fn filter_popup_move_field_resets_focused_option() {
        let mut p = FilterPopup {
            focused_option: 2,
            ..FilterPopup::default()
        };
        p.move_field_down();
        assert_eq!(p.focused_option, 0);
    }

    #[test]
    fn filter_popup_move_option_down_cycles_within_field() {
        let mut p = FilterPopup::default(); // PolicyStatus has 3 options
        let opts_len = FilterField::PolicyStatus.options().len();
        for i in 0..opts_len {
            assert_eq!(p.focused_option, i);
            p.move_option_down(opts_len);
        }
        assert_eq!(p.focused_option, 0); // wrapped back
    }

    #[test]
    fn filter_popup_move_option_up_cycles_within_field() {
        let mut p = FilterPopup::default();
        let opts_len = FilterField::PolicyStatus.options().len();
        p.move_option_up(opts_len); // should wrap to last option
        assert_eq!(p.focused_option, opts_len - 1);
    }

    // ------------------------------------------------------------------
    // StatefulList
    // ------------------------------------------------------------------

    #[test]
    fn stateful_list_new_selects_first() {
        let list = StatefulList::new(vec![1, 2, 3]);
        assert_eq!(list.selected, 0);
        assert_eq!(list.selected_item(), Some(&1));
    }

    #[test]
    fn stateful_list_default_is_empty() {
        let list: StatefulList<i32> = StatefulList::default();
        assert!(list.is_empty());
        assert_eq!(list.selected_item(), None);
    }

    #[test]
    fn stateful_list_selected_item_respects_index() {
        let mut list = StatefulList::new(vec!["a", "b", "c"]);
        list.selected = 2;
        assert_eq!(list.selected_item(), Some(&"c"));
    }

    #[test]
    fn stateful_list_selected_item_out_of_bounds_returns_none() {
        let mut list = StatefulList::new(vec!["a"]);
        list.selected = 99;
        assert_eq!(list.selected_item(), None);
    }

    // ------------------------------------------------------------------
    // App::clamp_selection_to_filter
    // ------------------------------------------------------------------

    #[test]
    fn clamp_keeps_selection_when_still_visible() {
        let components = vec![
            make_component("a", Some("IN_VIOLATION"), None, None),
            make_component("b", Some("IN_VIOLATION"), None, None),
            make_component("c", Some("NOT_IN_VIOLATION"), None, None),
        ];
        let mut app = make_app_with_components(components);
        app.filter.policy_statuses.push("IN_VIOLATION".to_string());
        // "a" and "b" are at raw indices 0 and 1, both pass filter
        app.components.selected = 1; // "b" — still visible
        app.clamp_selection_to_filter();
        assert_eq!(app.components.selected, 1); // unchanged
    }

    #[test]
    fn clamp_snaps_to_first_when_selected_filtered_out() {
        let components = vec![
            make_component("a", Some("IN_VIOLATION"), None, None),
            make_component("b", Some("NOT_IN_VIOLATION"), None, None),
            make_component("c", Some("IN_VIOLATION"), None, None),
        ];
        let mut app = make_app_with_components(components);
        // "b" at index 1 is selected but will be filtered out
        app.components.selected = 1;
        app.filter.policy_statuses.push("IN_VIOLATION".to_string());
        app.clamp_selection_to_filter();
        // first visible index is 0 ("a")
        assert_eq!(app.components.selected, 0);
    }

    #[test]
    fn clamp_to_zero_when_nothing_passes_filter() {
        let components = vec![
            make_component("a", Some("NOT_IN_VIOLATION"), None, None),
            make_component("b", Some("NOT_IN_VIOLATION"), None, None),
        ];
        let mut app = make_app_with_components(components);
        app.components.selected = 1;
        // filter that matches nothing
        app.filter.policy_statuses.push("IN_VIOLATION".to_string());
        app.clamp_selection_to_filter();
        assert_eq!(app.components.selected, 0); // defaults to 0
    }

    #[test]
    fn clamp_policy_violations_independently() {
        let violations = vec![
            make_component("x", Some("IN_VIOLATION"), Some("REVIEWED"), None),
            make_component("y", Some("IN_VIOLATION"), Some("UNREVIEWED"), None),
        ];
        let mut app = App::new(crate::config::Config::default());
        app.policy_violations = StatefulList::new(violations);
        // select "y" at raw index 1, then filter to REVIEWED only → "y" hidden
        app.policy_violations.selected = 1;
        app.filter.review_statuses.push("REVIEWED".to_string());
        app.clamp_selection_to_filter();
        // first (and only) visible policy violation is index 0 ("x")
        assert_eq!(app.policy_violations.selected, 0);
    }

    // ------------------------------------------------------------------
    // App::filtered_components / filtered_policy_violations
    // ------------------------------------------------------------------

    fn make_app_with_components(components: Vec<BomComponent>) -> App {
        let mut app = App::new(crate::config::Config::default());
        app.components = StatefulList::new(components);
        app
    }

    #[test]
    fn filtered_components_no_filter_returns_all() {
        let components = vec![
            make_component("alpha", None, None, None),
            make_component("beta", None, None, None),
        ];
        let app = make_app_with_components(components);
        assert_eq!(app.filtered_components().len(), 2);
    }

    #[test]
    fn filtered_components_search_filters_by_name() {
        let components = vec![
            make_component("alpha", None, None, None),
            make_component("beta", None, None, None),
        ];
        let mut app = make_app_with_components(components);
        app.search_input = "alph".to_string();
        let result = app.filtered_components();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.component_name, "alpha");
    }

    #[test]
    fn filtered_components_status_filter_applied() {
        let components = vec![
            make_component("a", Some("IN_VIOLATION"), None, None),
            make_component("b", Some("NOT_IN_VIOLATION"), None, None),
        ];
        let mut app = make_app_with_components(components);
        app.filter.policy_statuses.push("IN_VIOLATION".to_string());
        let result = app.filtered_components();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.component_name, "a");
    }

    #[test]
    fn filtered_policy_violations_ignores_policy_status_filter() {
        let violations = vec![
            make_component("a", Some("IN_VIOLATION"), Some("REVIEWED"), None),
            make_component("b", Some("IN_VIOLATION"), Some("UNREVIEWED"), None),
        ];
        let mut app = App::new(crate::config::Config::default());
        app.policy_violations = StatefulList::new(violations);
        // Even if policy_status filter is set, it should be ignored for violations tab
        app.filter
            .policy_statuses
            .push("NOT_IN_VIOLATION".to_string());
        let result = app.filtered_policy_violations();
        // Both components should still appear (policy_status filter ignored)
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filtered_policy_violations_review_filter_applied() {
        let violations = vec![
            make_component("a", Some("IN_VIOLATION"), Some("REVIEWED"), None),
            make_component("b", Some("IN_VIOLATION"), Some("UNREVIEWED"), None),
        ];
        let mut app = App::new(crate::config::Config::default());
        app.policy_violations = StatefulList::new(violations);
        app.filter.review_statuses.push("REVIEWED".to_string());
        let result = app.filtered_policy_violations();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.component_name, "a");
    }

    // ------------------------------------------------------------------
    // App::go_back — filter reset + pagination reset
    // ------------------------------------------------------------------

    #[test]
    fn go_back_from_components_resets_filter() {
        let mut app = App::new(crate::config::Config::default());
        app.screen = Screen::Components;
        app.filter.policy_statuses.push("IN_VIOLATION".to_string());
        app.filter_popup.open = true;
        app.go_back();
        assert!(app.filter.is_empty());
        assert!(!app.filter_popup.open);
    }

    #[test]
    fn go_back_from_components_resets_pagination_fields() {
        let mut app = App::new(crate::config::Config::default());
        app.screen = Screen::Components;
        app.components_total = 500;
        app.components_offset = 100;
        app.vulnerabilities_total = 200;
        app.vulnerabilities_offset = 100;
        app.policy_violations_total = 50;
        app.policy_violations_offset = 50;
        app.go_back();
        assert_eq!(app.components_total, 0);
        assert_eq!(app.components_offset, 0);
        assert_eq!(app.vulnerabilities_total, 0);
        assert_eq!(app.vulnerabilities_offset, 0);
        assert_eq!(app.policy_violations_total, 0);
        assert_eq!(app.policy_violations_offset, 0);
    }

    #[test]
    fn go_back_from_versions_does_not_reset_filter() {
        // Filter is only reset one level below (version-detail screens)
        let mut app = App::new(crate::config::Config::default());
        app.screen = Screen::Versions;
        app.filter.policy_statuses.push("IN_VIOLATION".to_string());
        app.go_back();
        // filter state is not touched by go_back from Versions
        assert!(!app.filter.is_empty());
    }

    // ------------------------------------------------------------------
    // VersionTab
    // ------------------------------------------------------------------

    #[test]
    fn version_tab_next_cycles() {
        assert_eq!(VersionTab::Components.next(), VersionTab::Vulnerabilities);
        assert_eq!(
            VersionTab::Vulnerabilities.next(),
            VersionTab::PolicyViolations
        );
        assert_eq!(VersionTab::PolicyViolations.next(), VersionTab::Components);
    }

    #[test]
    fn version_tab_prev_cycles() {
        assert_eq!(VersionTab::Components.prev(), VersionTab::PolicyViolations);
        assert_eq!(VersionTab::Vulnerabilities.prev(), VersionTab::Components);
        assert_eq!(
            VersionTab::PolicyViolations.prev(),
            VersionTab::Vulnerabilities
        );
    }

    #[test]
    fn version_tab_title_non_empty() {
        for tab in [
            VersionTab::Components,
            VersionTab::Vulnerabilities,
            VersionTab::PolicyViolations,
        ] {
            assert!(!tab.title().is_empty());
        }
    }
}
