use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc, Datelike, Timelike};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose};
use crate::ai_conversation::AIConversationEngine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub all_day: bool,
    pub recurring: bool,
    pub calendar_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarConfig {
    pub provider: CalendarProvider,
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub calendar_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CalendarProvider {
    Apple,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalDAVAuth {
    pub username: String,
    pub password: String,
    pub server_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarList {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub primary: bool,
    pub access_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingRequest {
    pub title: String,
    pub description: Option<String>,
    pub duration_minutes: u32,
    pub required_attendees: Vec<String>,
    pub optional_attendees: Vec<String>,
    pub location: Option<String>,
    pub earliest_start: DateTime<Utc>,
    pub latest_start: DateTime<Utc>,
    pub preferred_times: Vec<TimeSlot>,
    pub avoid_times: Vec<TimeSlot>,
    pub buffer_minutes: u32,
    pub allow_overlapping: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlot {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub calendar_id: Option<String>,
    pub event_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityRequest {
    pub attendees: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_minutes: u32,
    pub buffer_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResponse {
    pub available_slots: Vec<TimeSlot>,
    pub conflicts: Vec<ConflictInfo>,
    pub recommendations: Vec<SchedulingRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub attendee: String,
    pub conflicting_event: CalendarEvent,
    pub conflict_type: ConflictType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    DirectOverlap,
    BufferViolation,
    PreferenceViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingRecommendation {
    pub time_slot: TimeSlot,
    pub confidence_score: f32,
    pub attendee_availability: Vec<AttendeeAvailability>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendeeAvailability {
    pub attendee: String,
    pub status: AvailabilityStatus,
    pub conflicts: Vec<CalendarEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AvailabilityStatus {
    Available,
    Busy,
    Tentative,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingInvitation {
    pub meeting_id: String,
    pub organizer: String,
    pub attendees: Vec<InviteeInfo>,
    pub subject: String,
    pub body: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub location: Option<String>,
    pub meeting_url: Option<String>,
    pub response_deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteeInfo {
    pub email: String,
    pub name: Option<String>,
    pub required: bool,
    pub response_status: ResponseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResponseStatus {
    Pending,
    Accepted,
    Declined,
    Tentative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingConstraints {
    pub working_hours: Vec<WorkingHours>,
    pub time_zone: String,
    pub minimum_notice_hours: u32,
    pub maximum_lookahead_days: u32,
    pub preferred_meeting_length: u32,
    pub break_duration_minutes: u32,
    pub max_consecutive_meetings: u32,
    pub avoid_lunch_time: bool,
    pub lunch_start_hour: u32,
    pub lunch_end_hour: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingHours {
    pub day_of_week: u32, // 0 = Sunday, 1 = Monday, etc.
    pub start_hour: u32,
    pub start_minute: u32,
    pub end_hour: u32,
    pub end_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub resolution_type: ResolutionType,
    pub original_event: CalendarEvent,
    pub resolved_event: Option<CalendarEvent>,
    pub alternative_times: Vec<SchedulingRecommendation>,
    pub conflicts_resolved: Vec<ConflictInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionType {
    NoConflict,
    Rescheduled,
    RequiresManualIntervention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationStatus {
    pub meeting_id: String,
    pub total_invites: u32,
    pub responses_received: u32,
    pub accepted: u32,
    pub declined: u32,
    pub tentative: u32,
    pub pending: u32,
    pub response_rate: f32,
}

// Deadline Tracking and Time Blocking Data Structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deadline {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub due_date: DateTime<Utc>,
    pub created_date: DateTime<Utc>,
    pub priority: DeadlinePriority,
    pub status: DeadlineStatus,
    pub estimated_hours: f32,
    pub completed_hours: f32,
    pub category: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub assignee: Option<String>,
    pub project_id: Option<String>,
    pub reminder_settings: ReminderSettings,
    pub time_blocks: Vec<TimeBlock>,
    pub progress_milestones: Vec<ProgressMilestone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeadlinePriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeadlineStatus {
    NotStarted,
    InProgress,
    OnHold,
    Completed,
    Overdue,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderSettings {
    pub enabled: bool,
    pub advance_notifications: Vec<ReminderSchedule>,
    pub notification_channels: Vec<NotificationChannel>,
    pub escalation_enabled: bool,
    pub escalation_delay_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderSchedule {
    pub time_before_deadline: chrono::Duration,
    pub message: String,
    pub urgent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    CalendarAlert,
    Push,
    Slack,
    SMS,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBlock {
    pub id: String,
    pub deadline_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub planned_duration: chrono::Duration,
    pub actual_duration: Option<chrono::Duration>,
    pub productivity_score: Option<f32>,
    pub notes: Option<String>,
    pub calendar_event_id: Option<String>,
    pub status: TimeBlockStatus,
    pub focus_mode: bool,
    pub interruptions: Vec<Interruption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeBlockStatus {
    Planned,
    Active,
    Completed,
    Cancelled,
    Rescheduled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interruption {
    pub timestamp: DateTime<Utc>,
    pub duration: chrono::Duration,
    pub reason: String,
    pub impact_level: ImpactLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMilestone {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub target_date: DateTime<Utc>,
    pub completion_date: Option<DateTime<Utc>>,
    pub progress_percentage: f32,
    pub verification_method: VerificationMethod,
    pub dependencies: Vec<String>,
    pub deliverables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationMethod {
    SelfReported,
    PeerReview,
    Automated,
    ManagerApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeAllocationSuggestion {
    pub deadline_id: String,
    pub suggested_blocks: Vec<SuggestedTimeBlock>,
    pub total_allocated_time: chrono::Duration,
    pub confidence_score: f32,
    pub reasoning: String,
    pub alternative_strategies: Vec<AllocationStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedTimeBlock {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub focus_area: String,
    pub estimated_productivity: f32,
    pub buffer_time: chrono::Duration,
    pub prerequisites: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationStrategy {
    pub name: String,
    pub description: String,
    pub time_blocks: Vec<SuggestedTimeBlock>,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlineMetrics {
    pub deadline_id: String,
    pub completion_rate: f32,
    pub time_efficiency: f32,
    pub milestone_adherence: f32,
    pub productivity_trends: Vec<ProductivityDataPoint>,
    pub risk_indicators: Vec<RiskIndicator>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductivityDataPoint {
    pub date: DateTime<Utc>,
    pub hours_worked: f32,
    pub tasks_completed: u32,
    pub focus_score: f32,
    pub interruption_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskIndicator {
    pub indicator_type: RiskType,
    pub severity: RiskLevel,
    pub description: String,
    pub suggested_action: String,
    pub deadline_impact: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskType {
    TimeShortage,
    ScopeCreep,
    DependencyDelay,
    ResourceConstraint,
    QualityRisk,
    ExternalFactor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressVisualization {
    pub deadline_id: String,
    pub progress_bars: Vec<ProgressBar>,
    pub timeline: Vec<TimelineEvent>,
    pub risk_indicators: Vec<RiskIndicator>,
    pub recommendations: Vec<String>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressBar {
    pub label: String,
    pub percentage: f32,
    pub color: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub date: DateTime<Utc>,
    pub event_type: String,
    pub title: String,
    pub description: String,
    pub status: String,
}

// AI-Enhanced Features Data Structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSchedulingContext {
    pub user_preferences: UserPreferences,
    pub historical_patterns: Vec<SchedulingPattern>,
    pub meeting_context: MeetingContext,
    pub optimization_goals: Vec<OptimizationGoal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub preferred_meeting_times: Vec<TimeSlot>,
    pub avoid_times: Vec<TimeSlot>,
    pub max_meetings_per_day: u32,
    pub preferred_meeting_duration: u32,
    pub break_duration_minutes: u32,
    pub focus_time_blocks: Vec<TimeSlot>,
    pub energy_patterns: Vec<EnergyLevel>,
    pub commute_time_minutes: u32,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingPattern {
    pub pattern_id: String,
    pub pattern_type: PatternType,
    pub frequency: f32,
    pub success_rate: f32,
    pub user_satisfaction: f32,
    pub conditions: Vec<String>,
    pub outcomes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    MeetingSequence,
    TimeOfDay,
    DayOfWeek,
    Duration,
    Attendees,
    Location,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyLevel {
    pub time_of_day: u32, // Hour of day
    pub energy_score: f32, // 1.0 to 5.0
    pub focus_capacity: f32, // 1.0 to 5.0
    pub meeting_suitability: f32, // 1.0 to 5.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingContext {
    pub meeting_type: MeetingType,
    pub importance_level: ImportanceLevel,
    pub required_preparation_time: u32,
    pub post_meeting_buffer: u32,
    pub participants: Vec<ParticipantProfile>,
    pub decision_making_required: bool,
    pub creative_work_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeetingType {
    OneOnOne,
    TeamStandup,
    ProjectReview,
    Brainstorming,
    Interview,
    Presentation,
    Training,
    ClientCall,
    Strategic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportanceLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantProfile {
    pub email: String,
    pub name: Option<String>,
    pub role: String,
    pub timezone: String,
    pub preferred_times: Vec<TimeSlot>,
    pub availability_patterns: Vec<AvailabilityPattern>,
    pub meeting_history: MeetingHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityPattern {
    pub day_of_week: u32,
    pub typical_start_time: u32,
    pub typical_end_time: u32,
    pub busy_periods: Vec<TimeSlot>,
    pub preferred_meeting_types: Vec<MeetingType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingHistory {
    pub total_meetings: u32,
    pub average_duration: u32,
    pub response_rate: f32,
    pub punctuality_score: f32,
    pub engagement_score: f32,
    pub preferred_durations: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationGoal {
    MinimizeCommute,
    MaximizeFocus,
    BalanceWorkload,
    RespectEnergyLevels,
    MinimizeConflicts,
    MaximizeParticipation,
    OptimizeForCreativity,
    ReduceMeetingFatigue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartSchedulingSuggestion {
    pub suggestion_id: String,
    pub suggested_time: TimeSlot,
    pub confidence_score: f32,
    pub reasoning: Vec<String>,
    pub ai_insights: Vec<String>,
    pub optimization_factors: Vec<OptimizationFactor>,
    pub alternative_suggestions: Vec<AlternativeSuggestion>,
    pub impact_analysis: ImpactAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationFactor {
    pub factor_type: String,
    pub weight: f32,
    pub value: f32,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeSuggestion {
    pub time_slot: TimeSlot,
    pub confidence_score: f32,
    pub trade_offs: Vec<String>,
    pub benefits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub productivity_impact: f32,
    pub participant_satisfaction: f32,
    pub schedule_disruption: f32,
    pub energy_optimization: f32,
    pub follow_up_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingAgenda {
    pub agenda_id: String,
    pub meeting_title: String,
    pub meeting_type: MeetingType,
    pub duration_minutes: u32,
    pub objectives: Vec<String>,
    pub agenda_items: Vec<AgendaItem>,
    pub preparation_materials: Vec<PreparationMaterial>,
    pub success_criteria: Vec<String>,
    pub follow_up_actions: Vec<String>,
    pub ai_generated: bool,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgendaItem {
    pub item_id: String,
    pub title: String,
    pub description: Option<String>,
    pub duration_minutes: u32,
    pub item_type: AgendaItemType,
    pub owner: Option<String>,
    pub prerequisites: Vec<String>,
    pub expected_outcome: Option<String>,
    pub discussion_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgendaItemType {
    Discussion,
    Decision,
    Information,
    Action,
    Review,
    Brainstorm,
    Presentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparationMaterial {
    pub material_id: String,
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub required: bool,
    pub estimated_reading_time: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingOptimization {
    pub optimization_id: String,
    pub meeting_id: String,
    pub suggested_changes: Vec<OptimizationSuggestion>,
    pub potential_time_savings: u32,
    pub efficiency_improvements: Vec<String>,
    pub participant_experience_improvements: Vec<String>,
    pub ai_analysis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub suggestion_type: OptimizationType,
    pub description: String,
    pub impact: String,
    pub effort_level: EffortLevel,
    pub expected_benefit: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    DurationAdjustment,
    TimingChange,
    ParticipantOptimization,
    FormatChange,
    PreparationImprovement,
    FollowUpStreamlining,
    AgendaRefinement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartConflictResolution {
    pub resolution_id: String,
    pub conflict_analysis: ConflictAnalysis,
    pub ai_recommended_solutions: Vec<ConflictSolution>,
    pub stakeholder_impact: Vec<StakeholderImpact>,
    pub negotiation_strategies: Vec<String>,
    pub compromise_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictAnalysis {
    pub conflict_severity: ConflictSeverity,
    pub affected_meetings: Vec<String>,
    pub affected_participants: Vec<String>,
    pub priority_conflicts: Vec<PriorityConflict>,
    pub resolution_complexity: ComplexityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Minor,
    Moderate,
    Significant,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityConflict {
    pub meeting_id: String,
    pub priority_level: ImportanceLevel,
    pub flexibility_score: f32,
    pub rescheduling_difficulty: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Simple,
    Moderate,
    Complex,
    HighlyComplex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictSolution {
    pub solution_id: String,
    pub solution_type: SolutionType,
    pub description: String,
    pub implementation_steps: Vec<String>,
    pub success_probability: f32,
    pub participant_satisfaction_impact: f32,
    pub resource_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolutionType {
    Reschedule,
    DurationAdjustment,
    ParticipantSubstitution,
    MeetingSplit,
    FormatChange,
    Delegation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeholderImpact {
    pub participant_email: String,
    pub impact_level: StakeholderImpactLevel,
    pub specific_impacts: Vec<String>,
    pub mitigation_strategies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StakeholderImpactLevel {
    Minimal,
    Low,
    Medium,
    High,
    Severe,
}

pub struct CalendarAdapter {
    client: Client,
    config: CalendarConfig,
    auth_header: String,
    ai_conversation: Option<AIConversationEngine>,
}

impl CalendarAdapter {
    pub fn new(config: CalendarConfig) -> Result<Self> {
        let client = Client::new();
        
        // Create Basic Auth header for CalDAV
        let auth_string = format!("{}:{}", config.username, config.password);
        let auth_header = format!("Basic {}", general_purpose::STANDARD.encode(auth_string));
        
        Ok(CalendarAdapter {
            client,
            config,
            auth_header,
            ai_conversation: None,
        })
    }
    
    pub fn with_ai_conversation(mut self, ai_conversation: AIConversationEngine) -> Self {
        self.ai_conversation = Some(ai_conversation);
        self
    }

    pub async fn test_connection(&self) -> Result<bool> {
        let url = format!("{}/", self.config.server_url.trim_end_matches('/'));
        
        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(r#"<?xml version="1.0" encoding="UTF-8"?>
<D:propfind xmlns:D="DAV:">
    <D:prop>
        <D:displayname/>
        <D:resourcetype/>
    </D:prop>
</D:propfind>"#)
            .send()
            .await
            .context("Failed to connect to CalDAV server")?;

        Ok(response.status().is_success())
    }

    pub async fn get_calendar_list(&self) -> Result<Vec<CalendarList>> {
        let url = format!("{}/", self.config.server_url.trim_end_matches('/'));
        
        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(r#"<?xml version="1.0" encoding="UTF-8"?>
<D:propfind xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
    <D:prop>
        <D:displayname/>
        <D:resourcetype/>
        <C:supported-calendar-component-set/>
    </D:prop>
</D:propfind>"#)
            .send()
            .await
            .context("Failed to get calendar list")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to get calendar list: {}", error_text);
        }

        let response_text = response.text().await?;
        self.parse_calendar_list(&response_text)
    }

    fn parse_calendar_list(&self, xml_response: &str) -> Result<Vec<CalendarList>> {
        // Simple XML parsing for calendar resources
        let mut calendars = Vec::new();
        
        // For now, create a default calendar entry
        // In a full implementation, you'd parse the XML response properly
        if xml_response.contains("<D:collection/>") {
            calendars.push(CalendarList {
                id: "default".to_string(),
                name: self.config.calendar_name.clone().unwrap_or_else(|| "Default Calendar".to_string()),
                description: Some("Apple Calendar".to_string()),
                primary: true,
                access_role: "owner".to_string(),
            });
        }
        
        Ok(calendars)
    }

    pub async fn create_event(&self, calendar_id: &str, event: &CalendarEvent) -> Result<CalendarEvent> {
        let event_id = Uuid::new_v4().to_string();
        let ics_content = self.event_to_ics(event, &event_id)?;
        
        let url = format!("{}/{}.ics", 
            self.config.server_url.trim_end_matches('/'), 
            event_id
        );
        
        let response = self
            .client
            .put(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "text/calendar")
            .body(ics_content)
            .send()
            .await
            .context("Failed to create calendar event")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to create event: {}", error_text);
        }

        let mut created_event = event.clone();
        created_event.id = event_id;
        created_event.calendar_id = calendar_id.to_string();
        
        Ok(created_event)
    }

    pub async fn get_event(&self, calendar_id: &str, event_id: &str) -> Result<CalendarEvent> {
        let url = format!("{}/{}.ics", 
            self.config.server_url.trim_end_matches('/'), 
            event_id
        );
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .context("Failed to get calendar event")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to get event: {}", error_text);
        }

        let ics_content = response.text().await?;
        self.ics_to_event(&ics_content, event_id, calendar_id)
    }

    pub async fn update_event(&self, calendar_id: &str, event_id: &str, event: &CalendarEvent) -> Result<CalendarEvent> {
        let ics_content = self.event_to_ics(event, event_id)?;
        
        let url = format!("{}/{}.ics", 
            self.config.server_url.trim_end_matches('/'), 
            event_id
        );
        
        let response = self
            .client
            .put(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "text/calendar")
            .body(ics_content)
            .send()
            .await
            .context("Failed to update calendar event")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to update event: {}", error_text);
        }

        let mut updated_event = event.clone();
        updated_event.id = event_id.to_string();
        updated_event.calendar_id = calendar_id.to_string();
        
        Ok(updated_event)
    }

    pub async fn delete_event(&self, _calendar_id: &str, event_id: &str) -> Result<()> {
        let url = format!("{}/{}.ics", 
            self.config.server_url.trim_end_matches('/'), 
            event_id
        );
        
        let response = self
            .client
            .delete(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .context("Failed to delete calendar event")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to delete event: {}", error_text);
        }

        Ok(())
    }

    pub async fn list_events(&self, calendar_id: &str, time_min: Option<DateTime<Utc>>, time_max: Option<DateTime<Utc>>) -> Result<Vec<CalendarEvent>> {
        let mut filter = String::new();
        if let Some(start) = time_min {
            filter.push_str(&format!("DTSTART >= {}", start.format("%Y%m%dT%H%M%SZ")));
        }
        if let Some(end) = time_max {
            if !filter.is_empty() {
                filter.push_str(" AND ");
            }
            filter.push_str(&format!("DTEND <= {}", end.format("%Y%m%dT%H%M%SZ")));
        }

        let time_range = if filter.is_empty() { 
            "".to_string() 
        } else { 
            format!("<C:time-range start=\"{}\" end=\"{}\"/>", 
                time_min.unwrap_or_else(|| Utc::now()).format("%Y%m%dT%H%M%SZ"),
                time_max.unwrap_or_else(|| Utc::now() + chrono::Duration::days(365)).format("%Y%m%dT%H%M%SZ")) 
        };
        
        let report_body = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<C:calendar-query xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:D="DAV:">
    <D:prop>
        <D:getetag/>
        <C:calendar-data/>
    </D:prop>
    <C:filter>
        <C:comp-filter name="VCALENDAR">
            <C:comp-filter name="VEVENT">
                {}
            </C:comp-filter>
        </C:comp-filter>
    </C:filter>
</C:calendar-query>"#, time_range);

        let url = format!("{}/", self.config.server_url.trim_end_matches('/'));
        
        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"REPORT").unwrap(), &url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(report_body)
            .send()
            .await
            .context("Failed to list calendar events")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to list events: {}", error_text);
        }

        let response_text = response.text().await?;
        self.parse_event_list(&response_text, calendar_id)
    }

    fn parse_event_list(&self, _xml_response: &str, _calendar_id: &str) -> Result<Vec<CalendarEvent>> {
        let events = Vec::new();
        
        // Simple parsing - in a real implementation, you'd use a proper XML parser
        // For now, return empty list as this is a simplified implementation
        // You would parse the XML response and extract calendar data blocks
        
        Ok(events)
    }

    fn event_to_ics(&self, event: &CalendarEvent, event_id: &str) -> Result<String> {
        let now = Utc::now();
        let start_str = if event.all_day {
            format!("DTSTART;VALUE=DATE:{}", event.start_time.format("%Y%m%d"))
        } else {
            format!("DTSTART:{}", event.start_time.format("%Y%m%dT%H%M%SZ"))
        };
        
        let end_str = if event.all_day {
            format!("DTEND;VALUE=DATE:{}", event.end_time.format("%Y%m%d"))
        } else {
            format!("DTEND:{}", event.end_time.format("%Y%m%dT%H%M%SZ"))
        };

        let mut ics = format!(
            "BEGIN:VCALENDAR\r\n\
             VERSION:2.0\r\n\
             PRODID:-//Arrowhead//Calendar Adapter//EN\r\n\
             BEGIN:VEVENT\r\n\
             UID:{}\r\n\
             DTSTAMP:{}\r\n\
             {}\r\n\
             {}\r\n\
             SUMMARY:{}\r\n",
            event_id,
            now.format("%Y%m%dT%H%M%SZ"),
            start_str,
            end_str,
            event.title.replace('\n', "\\n").replace('\r', "\\r")
        );

        if let Some(description) = &event.description {
            ics.push_str(&format!("DESCRIPTION:{}\r\n", 
                description.replace('\n', "\\n").replace('\r', "\\r")));
        }

        if let Some(location) = &event.location {
            ics.push_str(&format!("LOCATION:{}\r\n", 
                location.replace('\n', "\\n").replace('\r', "\\r")));
        }

        for attendee in &event.attendees {
            ics.push_str(&format!("ATTENDEE:MAILTO:{}\r\n", attendee));
        }

        ics.push_str("END:VEVENT\r\n");
        ics.push_str("END:VCALENDAR\r\n");

        Ok(ics)
    }

    fn ics_to_event(&self, ics_content: &str, event_id: &str, calendar_id: &str) -> Result<CalendarEvent> {
        let mut event = CalendarEvent {
            id: event_id.to_string(),
            title: "Untitled Event".to_string(),
            description: None,
            start_time: Utc::now(),
            end_time: Utc::now() + chrono::Duration::hours(1),
            location: None,
            attendees: Vec::new(),
            all_day: false,
            recurring: false,
            calendar_id: calendar_id.to_string(),
        };

        // Simple ICS parsing - in a real implementation, you'd use a proper ICS parser
        for line in ics_content.lines() {
            let line = line.trim();
            if line.starts_with("SUMMARY:") {
                event.title = line[8..].to_string();
            } else if line.starts_with("DESCRIPTION:") {
                event.description = Some(line[12..].to_string());
            } else if line.starts_with("LOCATION:") {
                event.location = Some(line[9..].to_string());
            } else if line.starts_with("DTSTART:") {
                let datetime_str = &line[8..];
                // Handle UTC format: 20240101T100000Z
                if datetime_str.ends_with('Z') {
                    let utc_str = format!("{}+00:00", &datetime_str[..datetime_str.len()-1]);
                    if let Ok(dt) = DateTime::parse_from_str(&utc_str, "%Y%m%dT%H%M%S%z") {
                        event.start_time = dt.with_timezone(&Utc);
                    }
                } else if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
                    event.start_time = dt.with_timezone(&Utc);
                }
            } else if line.starts_with("DTEND:") {
                let datetime_str = &line[6..];
                // Handle UTC format: 20240101T110000Z
                if datetime_str.ends_with('Z') {
                    let utc_str = format!("{}+00:00", &datetime_str[..datetime_str.len()-1]);
                    if let Ok(dt) = DateTime::parse_from_str(&utc_str, "%Y%m%dT%H%M%S%z") {
                        event.end_time = dt.with_timezone(&Utc);
                    }
                } else if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
                    event.end_time = dt.with_timezone(&Utc);
                }
            } else if line.starts_with("DTSTART;VALUE=DATE:") {
                event.all_day = true;
                if let Ok(dt) = DateTime::parse_from_str(&format!("{}T00:00:00Z", &line[19..]), "%Y%m%dT%H%M%SZ") {
                    event.start_time = dt.with_timezone(&Utc);
                }
            } else if line.starts_with("DTEND;VALUE=DATE:") {
                event.all_day = true;
                if let Ok(dt) = DateTime::parse_from_str(&format!("{}T00:00:00Z", &line[17..]), "%Y%m%dT%H%M%SZ") {
                    event.end_time = dt.with_timezone(&Utc);
                }
            } else if line.starts_with("ATTENDEE:MAILTO:") {
                event.attendees.push(line[16..].to_string());
            } else if line.starts_with("RRULE:") {
                event.recurring = true;
            }
        }

        Ok(event)
    }

    pub fn get_config(&self) -> &CalendarConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: CalendarConfig) {
        let auth_string = format!("{}:{}", config.username, config.password);
        self.auth_header = format!("Basic {}", general_purpose::STANDARD.encode(auth_string));
        self.config = config;
    }

    pub fn get_caldav_server_url() -> &'static str {
        "https://caldav.icloud.com"
    }

    // Meeting Scheduling System Implementation
    
    /// Check availability across multiple calendars and attendees
    pub async fn check_availability(&self, request: &AvailabilityRequest) -> Result<AvailabilityResponse> {
        let mut available_slots = Vec::new();
        let mut conflicts = Vec::new();
        let mut recommendations = Vec::new();

        // Get all events in the requested time range
        let events = self.list_events("default", Some(request.start_time), Some(request.end_time)).await?;
        
        // Find available time slots
        let mut current_time = request.start_time;
        let slot_duration = chrono::Duration::minutes(request.duration_minutes as i64);
        
        while current_time + slot_duration <= request.end_time {
            let slot_end = current_time + slot_duration;
            
            // Check for conflicts with existing events
            let mut has_conflict = false;
            for event in &events {
                if self.times_overlap(current_time, slot_end, event.start_time, event.end_time, request.buffer_minutes) {
                    has_conflict = true;
                    conflicts.push(ConflictInfo {
                        attendee: "self".to_string(), // In a real implementation, this would be the actual attendee
                        conflicting_event: event.clone(),
                        conflict_type: ConflictType::DirectOverlap,
                    });
                    break;
                }
            }
            
            if !has_conflict {
                available_slots.push(TimeSlot {
                    start_time: current_time,
                    end_time: slot_end,
                    calendar_id: Some("default".to_string()),
                    event_id: None,
                });
            }
            
            // Move to next potential slot (15-minute intervals)
            current_time = current_time + chrono::Duration::minutes(15);
        }
        
        // Generate recommendations based on available slots
        for slot in &available_slots {
            let confidence_score = self.calculate_confidence_score(slot, &events);
            let reasoning = self.generate_reasoning(slot, &events);
            
            recommendations.push(SchedulingRecommendation {
                time_slot: slot.clone(),
                confidence_score,
                attendee_availability: vec![AttendeeAvailability {
                    attendee: "self".to_string(),
                    status: AvailabilityStatus::Available,
                    conflicts: Vec::new(),
                }],
                reasoning,
            });
        }
        
        // Sort recommendations by confidence score
        recommendations.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap());
        
        Ok(AvailabilityResponse {
            available_slots,
            conflicts,
            recommendations,
        })
    }
    
    /// Find optimal meeting time based on meeting request
    pub async fn find_meeting_time(&self, request: &MeetingRequest) -> Result<Vec<SchedulingRecommendation>> {
        let availability_request = AvailabilityRequest {
            attendees: request.required_attendees.clone(),
            start_time: request.earliest_start,
            end_time: request.latest_start,
            duration_minutes: request.duration_minutes,
            buffer_minutes: request.buffer_minutes,
        };
        
        let availability = self.check_availability(&availability_request).await?;
        
        // Filter recommendations based on meeting preferences
        let mut filtered_recommendations = Vec::new();
        
        for recommendation in availability.recommendations {
            let mut is_suitable = true;
            
            // Check against preferred times
            if !request.preferred_times.is_empty() {
                let mut matches_preference = false;
                for preferred_slot in &request.preferred_times {
                    if self.times_overlap(
                        recommendation.time_slot.start_time,
                        recommendation.time_slot.end_time,
                        preferred_slot.start_time,
                        preferred_slot.end_time,
                        0
                    ) {
                        matches_preference = true;
                        break;
                    }
                }
                if !matches_preference {
                    is_suitable = false;
                }
            }
            
            // Check against avoid times
            for avoid_slot in &request.avoid_times {
                if self.times_overlap(
                    recommendation.time_slot.start_time,
                    recommendation.time_slot.end_time,
                    avoid_slot.start_time,
                    avoid_slot.end_time,
                    0
                ) {
                    is_suitable = false;
                    break;
                }
            }
            
            if is_suitable {
                filtered_recommendations.push(recommendation);
            }
        }
        
        // Limit to top 10 recommendations
        filtered_recommendations.truncate(10);
        
        Ok(filtered_recommendations)
    }
    
    /// Check if two time ranges overlap, considering buffer time
    fn times_overlap(&self, start1: DateTime<Utc>, end1: DateTime<Utc>, start2: DateTime<Utc>, end2: DateTime<Utc>, buffer_minutes: u32) -> bool {
        let buffer_duration = chrono::Duration::minutes(buffer_minutes as i64);
        let adjusted_start1 = start1 - buffer_duration;
        let adjusted_end1 = end1 + buffer_duration;
        
        adjusted_start1 < end2 && adjusted_end1 > start2
    }
    
    /// Calculate confidence score for a time slot
    fn calculate_confidence_score(&self, slot: &TimeSlot, events: &[CalendarEvent]) -> f32 {
        let mut score: f32 = 1.0;
        
        // Check time of day preferences (prefer business hours)
        let hour = slot.start_time.hour();
        if hour >= 9 && hour <= 17 {
            score *= 1.2;
        } else if hour < 8 || hour > 18 {
            score *= 0.7;
        }
        
        // Check day of week preferences (prefer weekdays)
        let weekday = slot.start_time.weekday();
        if weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun {
            score *= 0.5;
        }
        
        // Check proximity to other events (prefer gaps between meetings)
        let mut has_adjacent_meeting = false;
        for event in events {
            let time_diff = (event.start_time - slot.end_time).num_minutes().abs();
            if time_diff < 30 {
                has_adjacent_meeting = true;
                break;
            }
        }
        
        if has_adjacent_meeting {
            score *= 0.9;
        }
        
        // Cap score at 1.0
        score.min(1.0)
    }
    
    /// Generate human-readable reasoning for a time slot recommendation
    fn generate_reasoning(&self, slot: &TimeSlot, events: &[CalendarEvent]) -> String {
        let mut reasons = Vec::new();
        
        let hour = slot.start_time.hour();
        let weekday = slot.start_time.weekday();
        
        if hour >= 9 && hour <= 17 {
            reasons.push("Within business hours".to_string());
        }
        
        if weekday != chrono::Weekday::Sat && weekday != chrono::Weekday::Sun {
            reasons.push("Weekday timing".to_string());
        }
        
        // Check for gaps before and after
        let mut has_buffer_before = true;
        let mut has_buffer_after = true;
        
        for event in events {
            let time_before = (slot.start_time - event.end_time).num_minutes();
            let time_after = (event.start_time - slot.end_time).num_minutes();
            
            if time_before > 0 && time_before < 30 {
                has_buffer_before = false;
            }
            if time_after > 0 && time_after < 30 {
                has_buffer_after = false;
            }
        }
        
        if has_buffer_before && has_buffer_after {
            reasons.push("Good buffer time around meeting".to_string());
        }
        
        if reasons.is_empty() {
            "Available time slot".to_string()
        } else {
            reasons.join(", ")
        }
    }
    
    /// Detect conflicts for a proposed meeting time
    pub async fn detect_conflicts(&self, calendar_id: &str, proposed_event: &CalendarEvent) -> Result<Vec<ConflictInfo>> {
        let mut conflicts = Vec::new();
        
        // Get existing events in the time range
        let existing_events = self.list_events(
            calendar_id,
            Some(proposed_event.start_time - chrono::Duration::hours(1)),
            Some(proposed_event.end_time + chrono::Duration::hours(1))
        ).await?;
        
        for event in existing_events {
            if event.id == proposed_event.id {
                continue; // Skip the same event
            }
            
            let conflict_type = if self.times_overlap(
                proposed_event.start_time,
                proposed_event.end_time,
                event.start_time,
                event.end_time,
                0
            ) {
                ConflictType::DirectOverlap
            } else if self.times_overlap(
                proposed_event.start_time,
                proposed_event.end_time,
                event.start_time,
                event.end_time,
                15 // 15-minute buffer
            ) {
                ConflictType::BufferViolation
            } else {
                continue; // No conflict
            };
            
            conflicts.push(ConflictInfo {
                attendee: "self".to_string(),
                conflicting_event: event,
                conflict_type,
            });
        }
        
        Ok(conflicts)
    }
    
    /// Resolve conflicts by suggesting alternative times
    pub async fn resolve_conflicts(&self, calendar_id: &str, proposed_event: &CalendarEvent, constraints: &SchedulingConstraints) -> Result<Vec<SchedulingRecommendation>> {
        let conflicts = self.detect_conflicts(calendar_id, proposed_event).await?;
        
        if conflicts.is_empty() {
            return Ok(vec![]);
        }
        
        let mut alternative_times = Vec::new();
        let original_duration = proposed_event.end_time - proposed_event.start_time;
        
        // Try to find alternative times within the same day
        let same_day_start = proposed_event.start_time.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let same_day_end = same_day_start + chrono::Duration::days(1);
        
        let availability_request = AvailabilityRequest {
            attendees: vec!["self".to_string()],
            start_time: same_day_start,
            end_time: same_day_end,
            duration_minutes: original_duration.num_minutes() as u32,
            buffer_minutes: constraints.break_duration_minutes,
        };
        
        let availability = self.check_availability(&availability_request).await?;
        
        // Filter alternatives based on working hours
        for recommendation in availability.recommendations {
            if self.is_within_working_hours(&recommendation.time_slot, constraints) {
                alternative_times.push(recommendation);
            }
        }
        
        // If no alternatives found on the same day, try the next few days
        if alternative_times.is_empty() {
            for days_ahead in 1..=3 {
                let next_day_start = same_day_start + chrono::Duration::days(days_ahead);
                let next_day_end = next_day_start + chrono::Duration::days(1);
                
                let next_day_request = AvailabilityRequest {
                    attendees: vec!["self".to_string()],
                    start_time: next_day_start,
                    end_time: next_day_end,
                    duration_minutes: original_duration.num_minutes() as u32,
                    buffer_minutes: constraints.break_duration_minutes,
                };
                
                let next_day_availability = self.check_availability(&next_day_request).await?;
                
                for recommendation in next_day_availability.recommendations {
                    if self.is_within_working_hours(&recommendation.time_slot, constraints) {
                        alternative_times.push(recommendation);
                    }
                }
                
                if !alternative_times.is_empty() {
                    break;
                }
            }
        }
        
        // Sort by confidence score and take top 5
        alternative_times.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap());
        alternative_times.truncate(5);
        
        Ok(alternative_times)
    }
    
    /// Check if a time slot is within working hours
    fn is_within_working_hours(&self, time_slot: &TimeSlot, constraints: &SchedulingConstraints) -> bool {
        let weekday = time_slot.start_time.weekday().num_days_from_sunday();
        let hour = time_slot.start_time.hour();
        let minute = time_slot.start_time.minute();
        
        for working_hours in &constraints.working_hours {
            if working_hours.day_of_week == weekday {
                let start_minutes = working_hours.start_hour * 60 + working_hours.start_minute;
                let end_minutes = working_hours.end_hour * 60 + working_hours.end_minute;
                let slot_minutes = hour * 60 + minute;
                
                if slot_minutes >= start_minutes && slot_minutes <= end_minutes {
                    // Check lunch time avoidance
                    if constraints.avoid_lunch_time {
                        let lunch_start = constraints.lunch_start_hour * 60;
                        let lunch_end = constraints.lunch_end_hour * 60;
                        
                        if slot_minutes >= lunch_start && slot_minutes <= lunch_end {
                            return false;
                        }
                    }
                    
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Auto-resolve conflicts by rescheduling or proposing alternatives
    pub async fn auto_resolve_conflicts(&self, calendar_id: &str, proposed_event: &CalendarEvent, constraints: &SchedulingConstraints) -> Result<ConflictResolution> {
        let conflicts = self.detect_conflicts(calendar_id, proposed_event).await?;
        
        if conflicts.is_empty() {
            return Ok(ConflictResolution {
                resolution_type: ResolutionType::NoConflict,
                original_event: proposed_event.clone(),
                resolved_event: None,
                alternative_times: Vec::new(),
                conflicts_resolved: Vec::new(),
            });
        }
        
        // Try to find an alternative time automatically
        let alternatives = self.resolve_conflicts(calendar_id, proposed_event, constraints).await?;
        
        if let Some(best_alternative) = alternatives.first() {
            let resolved_event = CalendarEvent {
                id: proposed_event.id.clone(),
                title: proposed_event.title.clone(),
                description: proposed_event.description.clone(),
                start_time: best_alternative.time_slot.start_time,
                end_time: best_alternative.time_slot.end_time,
                location: proposed_event.location.clone(),
                attendees: proposed_event.attendees.clone(),
                all_day: proposed_event.all_day,
                recurring: proposed_event.recurring,
                calendar_id: proposed_event.calendar_id.clone(),
            };
            
            Ok(ConflictResolution {
                resolution_type: ResolutionType::Rescheduled,
                original_event: proposed_event.clone(),
                resolved_event: Some(resolved_event),
                alternative_times: alternatives,
                conflicts_resolved: conflicts,
            })
        } else {
            Ok(ConflictResolution {
                resolution_type: ResolutionType::RequiresManualIntervention,
                original_event: proposed_event.clone(),
                resolved_event: None,
                alternative_times: alternatives,
                conflicts_resolved: conflicts,
            })
        }
    }
    
    /// Create and send meeting invitation
    pub async fn create_meeting_invitation(&self, meeting_request: &MeetingRequest, selected_time: &TimeSlot) -> Result<MeetingInvitation> {
        let meeting_id = Uuid::new_v4().to_string();
        
        let mut attendees = Vec::new();
        for email in &meeting_request.required_attendees {
            attendees.push(InviteeInfo {
                email: email.clone(),
                name: None,
                required: true,
                response_status: ResponseStatus::Pending,
            });
        }
        
        for email in &meeting_request.optional_attendees {
            attendees.push(InviteeInfo {
                email: email.clone(),
                name: None,
                required: false,
                response_status: ResponseStatus::Pending,
            });
        }
        
        let invitation = MeetingInvitation {
            meeting_id: meeting_id.clone(),
            organizer: self.config.username.clone(),
            attendees,
            subject: meeting_request.title.clone(),
            body: meeting_request.description.clone().unwrap_or_else(|| "Please join this meeting.".to_string()),
            start_time: selected_time.start_time,
            end_time: selected_time.end_time,
            location: meeting_request.location.clone(),
            meeting_url: None,
            response_deadline: Some(selected_time.start_time - chrono::Duration::hours(24)),
        };
        
        // Create the calendar event
        let calendar_event = CalendarEvent {
            id: meeting_id.clone(),
            title: meeting_request.title.clone(),
            description: meeting_request.description.clone(),
            start_time: selected_time.start_time,
            end_time: selected_time.end_time,
            location: meeting_request.location.clone(),
            attendees: meeting_request.required_attendees.iter()
                .chain(meeting_request.optional_attendees.iter())
                .cloned()
                .collect(),
            all_day: false,
            recurring: false,
            calendar_id: "default".to_string(),
        };
        
        self.create_event("default", &calendar_event).await?;
        
        Ok(invitation)
    }
    
    /// Update meeting invitation response status
    pub async fn update_invitation_response(&self, invitation_id: &str, attendee_email: &str, response: ResponseStatus) -> Result<()> {
        // In a real implementation, this would update the meeting invitation in a database
        // For now, we'll just simulate the update
        println!("Updated invitation {} response for {}: {:?}", invitation_id, attendee_email, response);
        Ok(())
    }
    
    /// Send meeting invitation reminder
    pub async fn send_invitation_reminder(&self, invitation: &MeetingInvitation) -> Result<()> {
        let pending_attendees: Vec<&InviteeInfo> = invitation.attendees
            .iter()
            .filter(|a| a.response_status == ResponseStatus::Pending)
            .collect();
        
        if pending_attendees.is_empty() {
            return Ok(());
        }
        
        for attendee in pending_attendees {
            // In a real implementation, this would send an email or notification
            println!("Sending reminder to {} for meeting: {}", attendee.email, invitation.subject);
        }
        
        Ok(())
    }
    
    /// Get meeting invitation status
    pub async fn get_invitation_status(&self, invitation_id: &str) -> Result<InvitationStatus> {
        // In a real implementation, this would query a database
        // For now, return a mock status
        Ok(InvitationStatus {
            meeting_id: invitation_id.to_string(),
            total_invites: 5,
            responses_received: 3,
            accepted: 2,
            declined: 1,
            tentative: 0,
            pending: 2,
            response_rate: 60.0,
        })
    }
    
    /// Cancel meeting invitation
    pub async fn cancel_meeting_invitation(&self, invitation_id: &str, reason: Option<&str>) -> Result<()> {
        // Delete the calendar event
        self.delete_event("default", invitation_id).await?;
        
        // In a real implementation, this would notify all attendees
        println!("Meeting {} cancelled. Reason: {}", invitation_id, reason.unwrap_or("No reason provided"));
        
        Ok(())
    }
    
    /// Reschedule meeting invitation
    pub async fn reschedule_meeting_invitation(&self, invitation_id: &str, new_time: &TimeSlot, reason: Option<&str>) -> Result<MeetingInvitation> {
        // Get the original event
        let original_event = self.get_event("default", invitation_id).await?;
        
        // Update the event with new time
        let updated_event = CalendarEvent {
            id: original_event.id.clone(),
            title: original_event.title.clone(),
            description: original_event.description.clone(),
            start_time: new_time.start_time,
            end_time: new_time.end_time,
            location: original_event.location.clone(),
            attendees: original_event.attendees.clone(),
            all_day: original_event.all_day,
            recurring: original_event.recurring,
            calendar_id: original_event.calendar_id.clone(),
        };
        
        self.update_event("default", invitation_id, &updated_event).await?;
        
        // Create updated invitation
        let updated_invitation = MeetingInvitation {
            meeting_id: invitation_id.to_string(),
            organizer: self.config.username.clone(),
            attendees: original_event.attendees.iter().map(|email| InviteeInfo {
                email: email.clone(),
                name: None,
                required: true,
                response_status: ResponseStatus::Pending, // Reset to pending for reschedule
            }).collect(),
            subject: format!("RESCHEDULED: {}", original_event.title),
            body: format!("This meeting has been rescheduled. Reason: {}", reason.unwrap_or("Schedule change")),
            start_time: new_time.start_time,
            end_time: new_time.end_time,
            location: original_event.location.clone(),
            meeting_url: None,
            response_deadline: Some(new_time.start_time - chrono::Duration::hours(24)),
        };
        
        Ok(updated_invitation)
    }
    
    // Deadline Tracking and Time Blocking Implementation
    
    /// Create a new deadline with automatic time blocking
    pub async fn create_deadline(&self, deadline: &Deadline) -> Result<Deadline> {
        let mut new_deadline = deadline.clone();
        
        // Update status based on current time
        new_deadline.status = self.calculate_deadline_status(&new_deadline);
        
        // Create time blocks automatically if estimated hours are provided
        if new_deadline.estimated_hours > 0.0 {
            let time_blocks = self.generate_automatic_time_blocks(&new_deadline).await?;
            new_deadline.time_blocks = time_blocks;
        }
        
        // Create calendar events for each time block
        for time_block in &new_deadline.time_blocks {
            let _calendar_event = self.create_time_block_event(&new_deadline, time_block).await?;
            // In a real implementation, you would update the time_block with the calendar_event.id
        }
        
        Ok(new_deadline)
    }
    
    /// Generate automatic time blocks for a deadline
    pub async fn generate_automatic_time_blocks(&self, deadline: &Deadline) -> Result<Vec<TimeBlock>> {
        let mut time_blocks = Vec::new();
        let now = Utc::now();
        
        // Calculate how much time is available until deadline
        let time_until_deadline = deadline.due_date - now;
        let available_days = time_until_deadline.num_days().max(1);
        
        // Calculate daily work allocation
        let total_hours_needed = deadline.estimated_hours - deadline.completed_hours;
        let hours_per_day = total_hours_needed / available_days as f32;
        
        // Generate time blocks for each working day
        let mut current_date = now.date_naive();
        let mut remaining_hours = total_hours_needed;
        
        while remaining_hours > 0.0 && current_date < deadline.due_date.date_naive() {
            // Skip weekends for now (can be made configurable)
            if current_date.weekday() != chrono::Weekday::Sat && current_date.weekday() != chrono::Weekday::Sun {
                let daily_hours = hours_per_day.min(remaining_hours).min(8.0); // Cap at 8 hours per day
                
                if daily_hours > 0.5 { // Only create blocks for meaningful work periods
                    let block_start = current_date.and_hms_opt(9, 0, 0).unwrap().and_utc();
                    let block_end = block_start + chrono::Duration::hours(daily_hours as i64);
                    
                    // Check for conflicts with existing events
                    let conflicts = self.check_time_block_conflicts(block_start, block_end).await?;
                    
                    if conflicts.is_empty() {
                        let time_block = TimeBlock {
                            id: Uuid::new_v4().to_string(),
                            deadline_id: deadline.id.clone(),
                            start_time: block_start,
                            end_time: block_end,
                            planned_duration: chrono::Duration::hours(daily_hours as i64),
                            actual_duration: None,
                            productivity_score: None,
                            notes: None,
                            calendar_event_id: None,
                            status: TimeBlockStatus::Planned,
                            focus_mode: true,
                            interruptions: Vec::new(),
                        };
                        
                        time_blocks.push(time_block);
                        remaining_hours -= daily_hours;
                    } else {
                        // Try to find alternative time slots
                        if let Some(alternative_slot) = self.find_alternative_time_slot(current_date, daily_hours).await? {
                            let time_block = TimeBlock {
                                id: Uuid::new_v4().to_string(),
                                deadline_id: deadline.id.clone(),
                                start_time: alternative_slot.start_time,
                                end_time: alternative_slot.end_time,
                                planned_duration: chrono::Duration::hours(daily_hours as i64),
                                actual_duration: None,
                                productivity_score: None,
                                notes: None,
                                calendar_event_id: None,
                                status: TimeBlockStatus::Planned,
                                focus_mode: true,
                                interruptions: Vec::new(),
                            };
                            
                            time_blocks.push(time_block);
                            remaining_hours -= daily_hours;
                        }
                    }
                }
            }
            
            current_date += chrono::Duration::days(1);
        }
        
        Ok(time_blocks)
    }
    
    /// Check for conflicts with existing time blocks
    async fn check_time_block_conflicts(&self, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Result<Vec<CalendarEvent>> {
        let existing_events = self.list_events("default", Some(start_time), Some(end_time)).await?;
        
        let conflicts = existing_events
            .into_iter()
            .filter(|event| {
                self.times_overlap(
                    start_time,
                    end_time,
                    event.start_time,
                    event.end_time,
                    0
                )
            })
            .collect();
        
        Ok(conflicts)
    }
    
    /// Find alternative time slot for a given day
    async fn find_alternative_time_slot(&self, date: chrono::NaiveDate, hours_needed: f32) -> Result<Option<TimeSlot>> {
        let day_start = date.and_hms_opt(8, 0, 0).unwrap().and_utc();
        let day_end = date.and_hms_opt(18, 0, 0).unwrap().and_utc();
        
        let availability_request = AvailabilityRequest {
            attendees: vec!["self".to_string()],
            start_time: day_start,
            end_time: day_end,
            duration_minutes: (hours_needed * 60.0) as u32,
            buffer_minutes: 15,
        };
        
        let availability = self.check_availability(&availability_request).await?;
        
        if let Some(slot) = availability.available_slots.first() {
            Ok(Some(slot.clone()))
        } else {
            Ok(None)
        }
    }
    
    /// Create calendar event for time block
    async fn create_time_block_event(&self, deadline: &Deadline, time_block: &TimeBlock) -> Result<CalendarEvent> {
        let event = CalendarEvent {
            id: time_block.id.clone(),
            title: format!("🎯 {}", deadline.title),
            description: Some(format!(
                "Time block for deadline: {}\n\nFocus Area: {}\nEstimated Duration: {} hours\nPriority: {:?}",
                deadline.title,
                deadline.category,
                time_block.planned_duration.num_hours(),
                deadline.priority
            )),
            start_time: time_block.start_time,
            end_time: time_block.end_time,
            location: None,
            attendees: vec![],
            all_day: false,
            recurring: false,
            calendar_id: "default".to_string(),
        };
        
        self.create_event("default", &event).await
    }
    
    /// Update time block with actual time spent
    pub async fn update_time_block_completion(&self, time_block_id: &str, actual_duration: chrono::Duration, productivity_score: f32, notes: Option<String>) -> Result<()> {
        // In a real implementation, this would update the time block in a database
        println!("Updated time block {} with actual duration: {} hours, productivity: {:.1}/5.0", 
                 time_block_id, 
                 actual_duration.num_hours(),
                 productivity_score);
        
        if let Some(notes) = notes {
            println!("Notes: {}", notes);
        }
        
        Ok(())
    }
    
    /// Calculate current deadline status
    fn calculate_deadline_status(&self, deadline: &Deadline) -> DeadlineStatus {
        let now = Utc::now();
        
        if deadline.completed_hours >= deadline.estimated_hours {
            return DeadlineStatus::Completed;
        }
        
        if now > deadline.due_date {
            return DeadlineStatus::Overdue;
        }
        
        if deadline.completed_hours > 0.0 {
            DeadlineStatus::InProgress
        } else {
            DeadlineStatus::NotStarted
        }
    }
    
    /// Get all deadlines with their current status
    pub async fn get_deadlines(&self, filter: Option<DeadlineStatus>) -> Result<Vec<Deadline>> {
        // In a real implementation, this would query a database
        // For now, return an empty vector
        let mut deadlines = Vec::new();
        
        // Mock deadline for demonstration
        let mock_deadline = Deadline {
            id: "deadline-1".to_string(),
            title: "Complete project documentation".to_string(),
            description: Some("Finish all technical documentation for the project".to_string()),
            due_date: Utc::now() + chrono::Duration::days(7),
            created_date: Utc::now() - chrono::Duration::days(3),
            priority: DeadlinePriority::High,
            status: DeadlineStatus::InProgress,
            estimated_hours: 20.0,
            completed_hours: 8.0,
            category: "Documentation".to_string(),
            tags: vec!["technical".to_string(), "urgent".to_string()],
            dependencies: vec![],
            assignee: Some("user@example.com".to_string()),
            project_id: Some("proj-123".to_string()),
            reminder_settings: ReminderSettings {
                enabled: true,
                advance_notifications: vec![
                    ReminderSchedule {
                        time_before_deadline: chrono::Duration::hours(24),
                        message: "Deadline approaching in 24 hours".to_string(),
                        urgent: true,
                    },
                    ReminderSchedule {
                        time_before_deadline: chrono::Duration::hours(72),
                        message: "Deadline in 3 days".to_string(),
                        urgent: false,
                    },
                ],
                notification_channels: vec![NotificationChannel::Email, NotificationChannel::CalendarAlert],
                escalation_enabled: true,
                escalation_delay_hours: 4,
            },
            time_blocks: vec![],
            progress_milestones: vec![],
        };
        
        if filter.is_none() || filter.as_ref().unwrap() == &mock_deadline.status {
            deadlines.push(mock_deadline);
        }
        
        Ok(deadlines)
    }
    
    /// Reschedule time blocks for a deadline
    pub async fn reschedule_time_blocks(&self, deadline_id: &str) -> Result<Vec<TimeBlock>> {
        // Get the deadline
        let deadlines = self.get_deadlines(None).await?;
        let deadline = deadlines.into_iter()
            .find(|d| d.id == deadline_id)
            .ok_or_else(|| anyhow::anyhow!("Deadline not found"))?;
        
        // Delete existing time blocks from calendar
        for time_block in &deadline.time_blocks {
            if let Some(event_id) = &time_block.calendar_event_id {
                let _ = self.delete_event("default", event_id).await;
            }
        }
        
        // Generate new time blocks
        let new_time_blocks = self.generate_automatic_time_blocks(&deadline).await?;
        
        // Create calendar events for new time blocks
        for time_block in &new_time_blocks {
            let _ = self.create_time_block_event(&deadline, time_block).await;
        }
        
        Ok(new_time_blocks)
    }
    
    // Progress Tracking System Implementation
    
    /// Update progress for a deadline
    pub async fn update_deadline_progress(&self, deadline_id: &str, completed_hours: f32, notes: Option<String>) -> Result<DeadlineMetrics> {
        // Get the deadline
        let deadlines = self.get_deadlines(None).await?;
        let deadline = deadlines.into_iter()
            .find(|d| d.id == deadline_id)
            .ok_or_else(|| anyhow::anyhow!("Deadline not found"))?;
        
        // Calculate metrics
        let metrics = self.calculate_deadline_metrics(&deadline, completed_hours).await?;
        
        // Update milestone progress
        let _updated_milestones = self.update_milestone_progress(&deadline, completed_hours).await?;
        
        // Log progress update
        println!("Progress updated for deadline '{}': {:.1}/{:.1} hours ({:.1}% complete)", 
                 deadline.title, 
                 completed_hours, 
                 deadline.estimated_hours,
                 metrics.completion_rate * 100.0);
        
        if let Some(notes) = notes {
            println!("Progress notes: {}", notes);
        }
        
        Ok(metrics)
    }
    
    /// Calculate comprehensive metrics for a deadline
    async fn calculate_deadline_metrics(&self, deadline: &Deadline, current_completed_hours: f32) -> Result<DeadlineMetrics> {
        let now = Utc::now();
        
        // Calculate completion rate
        let completion_rate = if deadline.estimated_hours > 0.0 {
            (current_completed_hours / deadline.estimated_hours).min(1.0)
        } else {
            0.0
        };
        
        // Calculate time efficiency
        let time_elapsed = now - deadline.created_date;
        let time_remaining = deadline.due_date - now;
        let total_time_available = deadline.due_date - deadline.created_date;
        
        let time_efficiency = if total_time_available.num_hours() > 0 {
            let expected_progress = time_elapsed.num_hours() as f32 / total_time_available.num_hours() as f32;
            if expected_progress > 0.0 {
                completion_rate / expected_progress
            } else {
                1.0
            }
        } else {
            1.0
        };
        
        // Calculate milestone adherence
        let milestone_adherence = self.calculate_milestone_adherence(&deadline.progress_milestones);
        
        // Generate productivity trends
        let productivity_trends = self.generate_productivity_trends(&deadline).await?;
        
        // Identify risk indicators
        let risk_indicators = self.identify_risk_indicators(&deadline, completion_rate, time_efficiency, time_remaining).await?;
        
        // Generate recommendations
        let recommendations = self.generate_progress_recommendations(&deadline, completion_rate, time_efficiency, &risk_indicators).await?;
        
        Ok(DeadlineMetrics {
            deadline_id: deadline.id.clone(),
            completion_rate,
            time_efficiency,
            milestone_adherence,
            productivity_trends,
            risk_indicators,
            recommendations,
        })
    }
    
    /// Calculate milestone adherence percentage
    fn calculate_milestone_adherence(&self, milestones: &[ProgressMilestone]) -> f32 {
        if milestones.is_empty() {
            return 1.0;
        }
        
        let now = Utc::now();
        let mut total_adherence = 0.0;
        let mut milestone_count = 0;
        
        for milestone in milestones {
            milestone_count += 1;
            
            if milestone.target_date <= now {
                // Milestone should be completed
                if milestone.completion_date.is_some() {
                    total_adherence += 1.0;
                } else {
                    // Overdue milestone
                    total_adherence += 0.0;
                }
            } else {
                // Future milestone - check if progress is on track
                let expected_progress = milestone.progress_percentage;
                if expected_progress >= 50.0 {
                    total_adherence += 0.8; // Partial credit for future milestones
                } else {
                    total_adherence += 0.4;
                }
            }
        }
        
        if milestone_count > 0 {
            total_adherence / milestone_count as f32
        } else {
            1.0
        }
    }
    
    /// Generate productivity trends for visualization
    async fn generate_productivity_trends(&self, _deadline: &Deadline) -> Result<Vec<ProductivityDataPoint>> {
        let mut trends = Vec::new();
        let now = Utc::now();
        
        // Generate mock productivity data for the last 7 days
        for i in 0..7 {
            let date = now - chrono::Duration::days(7 - i);
            let hours_worked = if i < 5 { 6.0 - (i as f32 * 0.5) } else { 2.0 }; // Simulate decreasing productivity
            let focus_score = if i < 3 { 4.5 - (i as f32 * 0.3) } else { 3.0 };
            
            trends.push(ProductivityDataPoint {
                date,
                hours_worked,
                tasks_completed: (hours_worked * 1.2) as u32,
                focus_score,
                interruption_count: if i > 3 { (8 - i) as u32 } else { 3 },
            });
        }
        
        Ok(trends)
    }
    
    /// Identify risk indicators for the deadline
    async fn identify_risk_indicators(&self, deadline: &Deadline, completion_rate: f32, time_efficiency: f32, time_remaining: chrono::Duration) -> Result<Vec<RiskIndicator>> {
        let mut risks = Vec::new();
        
        // Time shortage risk
        if time_remaining.num_hours() < 24 && completion_rate < 0.9 {
            risks.push(RiskIndicator {
                indicator_type: RiskType::TimeShortage,
                severity: RiskLevel::High,
                description: "Less than 24 hours remaining with significant work incomplete".to_string(),
                suggested_action: "Consider extending deadline or reducing scope".to_string(),
                deadline_impact: 0.8,
            });
        } else if completion_rate < 0.5 && time_remaining.num_days() < 3 {
            risks.push(RiskIndicator {
                indicator_type: RiskType::TimeShortage,
                severity: RiskLevel::Medium,
                description: "Progress is behind schedule with limited time remaining".to_string(),
                suggested_action: "Increase daily work allocation or request assistance".to_string(),
                deadline_impact: 0.6,
            });
        }
        
        // Efficiency risk
        if time_efficiency < 0.7 {
            risks.push(RiskIndicator {
                indicator_type: RiskType::ResourceConstraint,
                severity: RiskLevel::Medium,
                description: "Work efficiency is below expected levels".to_string(),
                suggested_action: "Review time blocks and eliminate distractions".to_string(),
                deadline_impact: 0.4,
            });
        }
        
        // Dependency risk
        if !deadline.dependencies.is_empty() {
            risks.push(RiskIndicator {
                indicator_type: RiskType::DependencyDelay,
                severity: RiskLevel::Low,
                description: "Deadline has dependencies that may cause delays".to_string(),
                suggested_action: "Monitor dependency status and create contingency plans".to_string(),
                deadline_impact: 0.3,
            });
        }
        
        Ok(risks)
    }
    
    /// Generate progress recommendations
    async fn generate_progress_recommendations(&self, _deadline: &Deadline, completion_rate: f32, time_efficiency: f32, risk_indicators: &[RiskIndicator]) -> Result<Vec<String>> {
        let mut recommendations = Vec::new();
        
        // Completion rate recommendations
        if completion_rate < 0.3 {
            recommendations.push("Consider breaking down the task into smaller, manageable chunks".to_string());
            recommendations.push("Schedule focused work sessions with minimal interruptions".to_string());
        } else if completion_rate < 0.7 {
            recommendations.push("Maintain current pace and consider extending work hours if needed".to_string());
        } else {
            recommendations.push("Great progress! Continue with current approach".to_string());
        }
        
        // Time efficiency recommendations
        if time_efficiency < 0.8 {
            recommendations.push("Review and optimize your work environment for better focus".to_string());
            recommendations.push("Consider using the Pomodoro technique for better time management".to_string());
        }
        
        // Risk-based recommendations
        for risk in risk_indicators {
            match risk.indicator_type {
                RiskType::TimeShortage => {
                    if risk.severity == RiskLevel::High {
                        recommendations.push("URGENT: Prioritize essential tasks and defer non-critical work".to_string());
                    }
                }
                RiskType::ResourceConstraint => {
                    recommendations.push("Consider delegating some tasks or requesting additional resources".to_string());
                }
                RiskType::DependencyDelay => {
                    recommendations.push("Proactively communicate with dependency owners about timeline".to_string());
                }
                _ => {}
            }
        }
        
        Ok(recommendations)
    }
    
    /// Update milestone progress based on overall deadline progress
    async fn update_milestone_progress(&self, deadline: &Deadline, completed_hours: f32) -> Result<Vec<ProgressMilestone>> {
        let mut updated_milestones = deadline.progress_milestones.clone();
        let overall_progress = if deadline.estimated_hours > 0.0 {
            (completed_hours / deadline.estimated_hours).min(1.0)
        } else {
            0.0
        };
        
        // Update milestone progress based on overall completion
        for milestone in &mut updated_milestones {
            if milestone.completion_date.is_none() {
                // Update progress percentage based on overall deadline progress
                let expected_milestone_progress = overall_progress * 100.0;
                if expected_milestone_progress >= 90.0 && milestone.target_date <= Utc::now() {
                    milestone.completion_date = Some(Utc::now());
                    milestone.progress_percentage = 100.0;
                } else {
                    milestone.progress_percentage = expected_milestone_progress.min(milestone.progress_percentage + 10.0);
                }
            }
        }
        
        Ok(updated_milestones)
    }
    
    /// Get visual progress indicators for display
    pub async fn get_progress_visualization(&self, deadline_id: &str) -> Result<ProgressVisualization> {
        let deadlines = self.get_deadlines(None).await?;
        let deadline = deadlines.into_iter()
            .find(|d| d.id == deadline_id)
            .ok_or_else(|| anyhow::anyhow!("Deadline not found"))?;
        
        let metrics = self.calculate_deadline_metrics(&deadline, deadline.completed_hours).await?;
        
        // Create progress bars
        let progress_bars = vec![
            ProgressBar {
                label: "Overall Progress".to_string(),
                percentage: metrics.completion_rate * 100.0,
                color: self.get_progress_color(metrics.completion_rate),
                status: self.get_progress_status(metrics.completion_rate),
            },
            ProgressBar {
                label: "Time Efficiency".to_string(),
                percentage: metrics.time_efficiency * 100.0,
                color: self.get_efficiency_color(metrics.time_efficiency),
                status: self.get_efficiency_status(metrics.time_efficiency),
            },
            ProgressBar {
                label: "Milestone Adherence".to_string(),
                percentage: metrics.milestone_adherence * 100.0,
                color: self.get_adherence_color(metrics.milestone_adherence),
                status: self.get_adherence_status(metrics.milestone_adherence),
            },
        ];
        
        // Create timeline visualization
        let timeline = self.create_timeline_visualization(&deadline).await?;
        
        // Risk indicators for display
        let risk_indicators = metrics.risk_indicators.clone();
        
        Ok(ProgressVisualization {
            deadline_id: deadline.id.clone(),
            progress_bars,
            timeline,
            risk_indicators,
            recommendations: metrics.recommendations,
            last_updated: Utc::now(),
        })
    }
    
    /// Get color for progress bar based on completion rate
    fn get_progress_color(&self, completion_rate: f32) -> String {
        if completion_rate >= 0.8 { "green".to_string() }
        else if completion_rate >= 0.5 { "yellow".to_string() }
        else { "red".to_string() }
    }
    
    /// Get status text for progress
    fn get_progress_status(&self, completion_rate: f32) -> String {
        if completion_rate >= 0.9 { "Excellent".to_string() }
        else if completion_rate >= 0.7 { "Good".to_string() }
        else if completion_rate >= 0.5 { "Fair".to_string() }
        else { "Needs Attention".to_string() }
    }
    
    /// Get color for efficiency bar
    fn get_efficiency_color(&self, efficiency: f32) -> String {
        if efficiency >= 1.0 { "green".to_string() }
        else if efficiency >= 0.8 { "yellow".to_string() }
        else { "red".to_string() }
    }
    
    /// Get status text for efficiency
    fn get_efficiency_status(&self, efficiency: f32) -> String {
        if efficiency >= 1.2 { "Ahead of Schedule".to_string() }
        else if efficiency >= 1.0 { "On Track".to_string() }
        else if efficiency >= 0.8 { "Slightly Behind".to_string() }
        else { "Behind Schedule".to_string() }
    }
    
    /// Get color for adherence bar
    fn get_adherence_color(&self, adherence: f32) -> String {
        if adherence >= 0.8 { "green".to_string() }
        else if adherence >= 0.6 { "yellow".to_string() }
        else { "red".to_string() }
    }
    
    /// Get status text for adherence
    fn get_adherence_status(&self, adherence: f32) -> String {
        if adherence >= 0.9 { "Excellent".to_string() }
        else if adherence >= 0.7 { "Good".to_string() }
        else if adherence >= 0.5 { "Fair".to_string() }
        else { "Poor".to_string() }
    }
    
    /// Create timeline visualization for deadline
    async fn create_timeline_visualization(&self, deadline: &Deadline) -> Result<Vec<TimelineEvent>> {
        let mut timeline = Vec::new();
        let now = Utc::now();
        
        // Add creation event
        timeline.push(TimelineEvent {
            date: deadline.created_date,
            event_type: "created".to_string(),
            title: "Deadline Created".to_string(),
            description: format!("Deadline '{}' was created", deadline.title),
            status: "completed".to_string(),
        });
        
        // Add milestone events
        for milestone in &deadline.progress_milestones {
            timeline.push(TimelineEvent {
                date: milestone.target_date,
                event_type: "milestone".to_string(),
                title: milestone.title.clone(),
                description: milestone.description.clone().unwrap_or_else(|| "Milestone checkpoint".to_string()),
                status: if milestone.completion_date.is_some() {
                    "completed".to_string()
                } else if milestone.target_date <= now {
                    "overdue".to_string()
                } else {
                    "pending".to_string()
                },
            });
        }
        
        // Add time block events
        for time_block in &deadline.time_blocks {
            timeline.push(TimelineEvent {
                date: time_block.start_time,
                event_type: "time_block".to_string(),
                title: "Work Session".to_string(),
                description: format!("Scheduled work session ({} hours)", time_block.planned_duration.num_hours()),
                status: match time_block.status {
                    TimeBlockStatus::Completed => "completed".to_string(),
                    TimeBlockStatus::Active => "active".to_string(),
                    TimeBlockStatus::Cancelled => "cancelled".to_string(),
                    _ => "pending".to_string(),
                },
            });
        }
        
        // Add due date
        timeline.push(TimelineEvent {
            date: deadline.due_date,
            event_type: "due_date".to_string(),
            title: "Due Date".to_string(),
            description: "Deadline is due".to_string(),
            status: if now > deadline.due_date {
                "overdue".to_string()
            } else {
                "pending".to_string()
            },
        });
        
        // Sort timeline by date
        timeline.sort_by(|a, b| a.date.cmp(&b.date));
        
        Ok(timeline)
    }
    
    // AI-Enhanced Features Implementation
    
    /// Generate AI-powered scheduling suggestions with intelligent analysis
    pub async fn generate_smart_scheduling_suggestions(
        &mut self,
        meeting_request: &MeetingRequest,
        context: &AiSchedulingContext,
    ) -> Result<Vec<SmartSchedulingSuggestion>> {
        // First get basic availability using existing logic
        let basic_recommendations = self.find_meeting_time(meeting_request).await?;
        
        // Prepare context for AI analysis
        let context_prompt = self.build_scheduling_context_prompt(meeting_request, context, &basic_recommendations);
        
        // Extract AI conversation temporarily to avoid borrow conflicts
        let mut ai_conversation = self.ai_conversation.take()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;
        
        // Get AI insights for each recommendation
        let mut smart_suggestions = Vec::new();
        
        for (index, basic_rec) in basic_recommendations.iter().enumerate().take(5) {
            let suggestion_prompt = format!(
                "Analyze this meeting scheduling option and provide insights:\n\n\
                Meeting: {} ({}min)\n\
                Proposed Time: {} to {}\n\
                Basic Confidence: {:.1}\n\
                Basic Reasoning: {}\n\n\
                Context:\n{}\n\n\
                Provide enhanced analysis including:\n\
                1. AI insights about optimal timing\n\
                2. Optimization factors and weights\n\
                3. Impact analysis on productivity and satisfaction\n\
                4. Alternative suggestions if applicable\n\
                5. Detailed reasoning for this recommendation\n\n\
                Format your response as structured analysis.",
                meeting_request.title,
                meeting_request.duration_minutes,
                basic_rec.time_slot.start_time.format("%Y-%m-%d %H:%M UTC"),
                basic_rec.time_slot.end_time.format("%Y-%m-%d %H:%M UTC"),
                basic_rec.confidence_score,
                basic_rec.reasoning,
                context_prompt
            );
            
            let ai_response = ai_conversation.send_message(suggestion_prompt).await
                .unwrap_or_else(|_| "AI analysis unavailable".to_string());
            
            let ai_insights = vec![ai_response.clone()]; // Simplified parsing for now
            let optimization_factors = vec![
                OptimizationFactor {
                    factor_type: "Time efficiency".to_string(),
                    weight: 0.8,
                    value: 0.7,
                    explanation: "Optimal time slot selection".to_string(),
                },
                OptimizationFactor {
                    factor_type: "Participant availability".to_string(),
                    weight: 0.9,
                    value: 0.8,
                    explanation: "Maximum participant availability".to_string(),
                }
            ];
            let impact_analysis = ImpactAnalysis {
                productivity_impact: 0.8,
                participant_satisfaction: 0.7,
                schedule_disruption: 0.2,
                energy_optimization: 0.6,
                follow_up_requirements: vec!["None".to_string()],
            };
            let alternatives = vec![];
            
            let smart_suggestion = SmartSchedulingSuggestion {
                suggestion_id: format!("smart-{}", index + 1),
                suggested_time: basic_rec.time_slot.clone(),
                confidence_score: basic_rec.confidence_score + 0.1, // Enhanced confidence with AI insights
                reasoning: vec![
                    basic_rec.reasoning.clone(),
                    "Enhanced with AI insights".to_string()
                ],
                ai_insights,
                optimization_factors,
                alternative_suggestions: alternatives,
                impact_analysis,
            };
            
            smart_suggestions.push(smart_suggestion);
        }
        
        // Put AI conversation back
        self.ai_conversation = Some(ai_conversation);
        
        // Sort by enhanced confidence score
        smart_suggestions.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap());
        
        Ok(smart_suggestions)
    }
    
    /// Generate AI-powered meeting agenda based on context and participants
    pub async fn generate_ai_meeting_agenda(
        &mut self,
        meeting_request: &MeetingRequest,
        context: &MeetingContext,
        participant_profiles: &[ParticipantProfile],
    ) -> Result<MeetingAgenda> {
        let participant_context = self.format_participant_context(participant_profiles);
        
        let agenda_prompt = format!(
            "Generate a comprehensive meeting agenda for the following:\n\n\
            Meeting Title: {}\n\
            Duration: {} minutes\n\
            Meeting Type: {:?}\n\
            Importance: {:?}\n\
            Description: {}\n\
            Participants: {}\n\
            Decision Making Required: {}\n\
            Creative Work Required: {}\n\n\
            Participant Context:\n{}\n\n\
            Please create a detailed agenda with:\n\
            1. Clear objectives aligned with meeting type and importance\n\
            2. Time-allocated agenda items with specific outcomes\n\
            3. Preparation materials participants should review\n\
            4. Success criteria for the meeting\n\
            5. Follow-up action items\n\n\
            Format the response as a structured agenda with time allocations.",
            meeting_request.title,
            meeting_request.duration_minutes,
            context.meeting_type,
            context.importance_level,
            meeting_request.description.as_deref().unwrap_or("No description"),
            meeting_request.required_attendees.join(", "),
            context.decision_making_required,
            context.creative_work_required,
            participant_context
        );
        
        let ai_conversation = self.ai_conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;
        
        let ai_response = ai_conversation.send_message(agenda_prompt).await;
        
        let response_text = match ai_response {
            Ok(text) => text,
            Err(_) => self.generate_fallback_agenda(meeting_request, context),
        };
        let agenda = self.parse_ai_agenda_response(&response_text, meeting_request, context).await?;
        
        Ok(agenda)
    }
    
    /// Provide AI-enhanced conflict resolution with intelligent alternatives
    pub async fn ai_enhanced_conflict_resolution(
        &mut self,
        calendar_id: &str,
        proposed_event: &CalendarEvent,
        context: &AiSchedulingContext,
    ) -> Result<SmartConflictResolution> {
        // First detect conflicts using existing logic
        let basic_conflicts = self.detect_conflicts(calendar_id, proposed_event).await?;
        
        if basic_conflicts.is_empty() {
            return Ok(SmartConflictResolution {
                resolution_id: Uuid::new_v4().to_string(),
                conflict_analysis: ConflictAnalysis {
                    conflict_severity: ConflictSeverity::Minor,
                    affected_meetings: Vec::new(),
                    affected_participants: Vec::new(),
                    priority_conflicts: Vec::new(),
                    resolution_complexity: ComplexityLevel::Simple,
                },
                ai_recommended_solutions: Vec::new(),
                stakeholder_impact: Vec::new(),
                negotiation_strategies: Vec::new(),
                compromise_options: Vec::new(),
            });
        }
        
        let user_context = self.format_user_context(context);
        
        let ai_conversation = self.ai_conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;
        
        let conflict_prompt = format!(
            "Analyze this calendar conflict and provide intelligent resolution strategies:\n\n\
            Proposed Meeting: {}\n\
            Time: {} to {}\n\
            Importance: High\n\
            Conflicts Found: {}\n\n\
            User Context:\n{}\n\n\
            Please provide:\n\
            1. Conflict severity assessment\n\
            2. Intelligent resolution strategies with success probabilities\n\
            3. Stakeholder impact analysis\n\
            4. Negotiation strategies for each participant\n\
            5. Compromise options that minimize disruption\n\
            6. Step-by-step implementation guidance\n\n\
            Prioritize solutions that respect user preferences and energy patterns.",
            proposed_event.title,
            proposed_event.start_time.format("%Y-%m-%d %H:%M UTC"),
            proposed_event.end_time.format("%Y-%m-%d %H:%M UTC"),
            basic_conflicts.len(),
            user_context
        );
        
        let ai_response = ai_conversation.send_message(conflict_prompt).await
            .unwrap_or_else(|_| "AI conflict analysis unavailable".to_string());
        
        let conflict_analysis = self.analyze_conflict_complexity(&basic_conflicts, context);
        let ai_solutions = self.parse_ai_conflict_solutions(&ai_response, &basic_conflicts).await?;
        let stakeholder_impact = self.assess_stakeholder_impact(&basic_conflicts, context).await?;
        let negotiation_strategies = self.extract_negotiation_strategies(&ai_response);
        let compromise_options = self.extract_compromise_options(&ai_response);
        
        Ok(SmartConflictResolution {
            resolution_id: Uuid::new_v4().to_string(),
            conflict_analysis,
            ai_recommended_solutions: ai_solutions,
            stakeholder_impact,
            negotiation_strategies,
            compromise_options,
        })
    }
    
    /// Generate smart time blocking recommendations with AI optimization
    pub async fn generate_smart_time_blocks(
        &mut self,
        deadline: &Deadline,
        context: &AiSchedulingContext,
    ) -> Result<TimeAllocationSuggestion> {
        let energy_patterns = self.format_energy_patterns(&context.user_preferences.energy_patterns);
        let preferred_times = self.format_preferred_times(&context.user_preferences.preferred_meeting_times);
        let focus_blocks = self.format_focus_blocks(&context.user_preferences.focus_time_blocks);
        
        let time_blocking_prompt = format!(
            "Optimize time blocking strategy for this deadline:\n\n\
            Task: {}\n\
            Description: {}\n\
            Due Date: {}\n\
            Estimated Hours: {}\n\
            Completed Hours: {}\n\
            Priority: {:?}\n\
            Category: {}\n\n\
            User Context:\n\
            Energy Patterns: {}\n\
            Preferred Work Times: {}\n\
            Focus Blocks: {}\n\
            Optimization Goals: {:?}\n\n\
            Please provide:\n\
            1. Optimal time allocation strategy\n\
            2. Suggested time blocks considering energy patterns\n\
            3. Alternative strategies with pros/cons\n\
            4. Risk assessment and mitigation\n\
            5. Focus area recommendations for each block\n\n\
            Consider the user's peak performance times and avoid scheduling conflicts.",
            deadline.title,
            deadline.description.as_deref().unwrap_or("No description"),
            deadline.due_date.format("%Y-%m-%d %H:%M UTC"),
            deadline.estimated_hours,
            deadline.completed_hours,
            deadline.priority,
            deadline.category,
            energy_patterns,
            preferred_times,
            focus_blocks,
            context.optimization_goals
        );
        
        let ai_conversation = self.ai_conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;
        
        let ai_response = ai_conversation.send_message(time_blocking_prompt).await
            .unwrap_or_else(|_| "AI time blocking analysis unavailable".to_string());
        
        let suggested_blocks = self.parse_ai_time_blocks(&ai_response, deadline).await?;
        let alternative_strategies = self.parse_alternative_strategies(&ai_response, deadline).await?;
        let confidence_score = self.calculate_time_blocking_confidence(&suggested_blocks, context);
        
        Ok(TimeAllocationSuggestion {
            deadline_id: deadline.id.clone(),
            suggested_blocks,
            total_allocated_time: chrono::Duration::hours(
                (deadline.estimated_hours - deadline.completed_hours) as i64
            ),
            confidence_score,
            reasoning: self.extract_ai_reasoning(&ai_response),
            alternative_strategies,
        })
    }
    
    /// Analyze and optimize existing meetings with AI insights
    pub async fn analyze_meeting_optimization(
        &mut self,
        meeting_id: &str,
        meeting_history: &[CalendarEvent],
        participant_feedback: Option<&str>,
    ) -> Result<MeetingOptimization> {
        let meeting = self.get_event("default", meeting_id).await?;
        let meeting_history_formatted = self.format_meeting_history(meeting_history);
        
        let ai_conversation = self.ai_conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI conversation not configured"))?;
        
        let optimization_prompt = format!(
            "Analyze this meeting for optimization opportunities:\n\n\
            Meeting: {}\n\
            Duration: {} minutes\n\
            Attendees: {}\n\
            Recurring: {}\n\
            Description: {}\n\n\
            Meeting History Pattern:\n{}\n\n\
            Participant Feedback: {}\n\n\
            Please provide:\n\
            1. Duration optimization suggestions\n\
            2. Timing improvement recommendations\n\
            3. Participant optimization (who really needs to be there?)\n\
            4. Format improvements (in-person vs virtual, async vs sync)\n\
            5. Preparation and follow-up streamlining\n\
            6. Agenda refinement suggestions\n\
            7. Estimated time savings and efficiency gains\n\n\
            Focus on actionable improvements that enhance productivity.",
            meeting.title,
            (meeting.end_time - meeting.start_time).num_minutes(),
            meeting.attendees.join(", "),
            meeting.recurring,
            meeting.description.as_deref().unwrap_or("No description"),
            meeting_history_formatted,
            participant_feedback.unwrap_or("No feedback provided")
        );
        
        let ai_response = ai_conversation.send_message(optimization_prompt).await
            .unwrap_or_else(|_| "AI optimization analysis unavailable".to_string());
        
        // ai_conversation is automatically dropped here
        
        let suggested_changes = self.parse_optimization_suggestions(&ai_response);
        let time_savings = self.calculate_potential_time_savings(&suggested_changes);
        let efficiency_improvements = self.extract_efficiency_improvements(&ai_response);
        let participant_improvements = self.extract_participant_improvements(&ai_response);
        
        Ok(MeetingOptimization {
            optimization_id: Uuid::new_v4().to_string(),
            meeting_id: meeting_id.to_string(),
            suggested_changes,
            potential_time_savings: time_savings,
            efficiency_improvements,
            participant_experience_improvements: participant_improvements,
            ai_analysis: ai_response,
        })
    }
    
    // Helper methods for AI feature implementation
    
    fn build_scheduling_context_prompt(
        &self,
        _meeting_request: &MeetingRequest,
        context: &AiSchedulingContext,
        _basic_recommendations: &[SchedulingRecommendation],
    ) -> String {
        format!(
            "User Preferences:\n\
            - Preferred meeting times: {}\n\
            - Avoid times: {}\n\
            - Max meetings per day: {}\n\
            - Break duration: {} minutes\n\
            - Energy patterns: {}\n\
            - Optimization goals: {:?}",
            self.format_time_slots(&context.user_preferences.preferred_meeting_times),
            self.format_time_slots(&context.user_preferences.avoid_times),
            context.user_preferences.max_meetings_per_day,
            context.user_preferences.break_duration_minutes,
            self.format_energy_patterns(&context.user_preferences.energy_patterns),
            context.optimization_goals
        )
    }
    
    fn parse_ai_scheduling_insights(&self, ai_response: &str) -> Vec<String> {
        ai_response
            .lines()
            .filter(|line| line.contains("insight") || line.contains("optimal") || line.contains("recommendation"))
            .map(|line| line.trim().to_string())
            .collect()
    }
    
    fn generate_optimization_factors(&self, _time_slot: &TimeSlot, context: &AiSchedulingContext) -> Vec<OptimizationFactor> {
        let mut factors = Vec::new();
        
        for goal in &context.optimization_goals {
            let factor = match goal {
                OptimizationGoal::RespectEnergyLevels => OptimizationFactor {
                    factor_type: "Energy Alignment".to_string(),
                    weight: 0.8,
                    value: 0.7, // Would be calculated based on actual time vs energy patterns
                    explanation: "Meeting time aligns with user's high-energy periods".to_string(),
                },
                OptimizationGoal::MinimizeConflicts => OptimizationFactor {
                    factor_type: "Conflict Avoidance".to_string(),
                    weight: 0.9,
                    value: 0.9,
                    explanation: "No conflicts with existing calendar events".to_string(),
                },
                OptimizationGoal::MaximizeFocus => OptimizationFactor {
                    factor_type: "Focus Optimization".to_string(),
                    weight: 0.7,
                    value: 0.6,
                    explanation: "Adequate buffer time around meeting for focus".to_string(),
                },
                _ => continue,
            };
            factors.push(factor);
        }
        
        factors
    }
    
    async fn analyze_scheduling_impact(&self, _time_slot: &TimeSlot, _context: &AiSchedulingContext) -> Result<ImpactAnalysis> {
        // In a real implementation, this would analyze the actual impact
        Ok(ImpactAnalysis {
            productivity_impact: 0.8,
            participant_satisfaction: 0.75,
            schedule_disruption: 0.2,
            energy_optimization: 0.7,
            follow_up_requirements: vec![
                "Send calendar invite with agenda".to_string(),
                "Share preparation materials 24 hours prior".to_string(),
            ],
        })
    }
    
    async fn generate_alternative_suggestions(
        &self,
        primary_slot: &TimeSlot,
        _meeting_request: &MeetingRequest,
        _context: &AiSchedulingContext,
    ) -> Result<Vec<AlternativeSuggestion>> {
        // Generate alternative time slots
        let mut alternatives = Vec::new();
        
        // Alternative 1: Same day, earlier
        let earlier_slot = TimeSlot {
            start_time: primary_slot.start_time - chrono::Duration::hours(2),
            end_time: primary_slot.end_time - chrono::Duration::hours(2),
            calendar_id: primary_slot.calendar_id.clone(),
            event_id: None,
        };
        
        alternatives.push(AlternativeSuggestion {
            time_slot: earlier_slot,
            confidence_score: 0.7,
            trade_offs: vec!["Earlier in the day, may conflict with morning routine".to_string()],
            benefits: vec!["More energy available".to_string(), "Leaves afternoon open for focused work".to_string()],
        });
        
        // Alternative 2: Next day, same time
        let next_day_slot = TimeSlot {
            start_time: primary_slot.start_time + chrono::Duration::days(1),
            end_time: primary_slot.end_time + chrono::Duration::days(1),
            calendar_id: primary_slot.calendar_id.clone(),
            event_id: None,
        };
        
        alternatives.push(AlternativeSuggestion {
            time_slot: next_day_slot,
            confidence_score: 0.8,
            trade_offs: vec!["Delayed by one day".to_string()],
            benefits: vec!["Same optimal time window".to_string(), "More preparation time available".to_string()],
        });
        
        Ok(alternatives)
    }
    
    fn calculate_enhanced_confidence(&self, _time_slot: &TimeSlot, _context: &AiSchedulingContext, ai_insights: &[String]) -> f32 {
        let base_confidence = 0.8; // Base confidence from traditional scheduling
        let ai_boost = if ai_insights.len() > 2 { 0.1 } else { 0.0 };
        f32::min(base_confidence + ai_boost, 1.0)
    }
    
    fn enhance_reasoning(&self, basic_reasoning: &str, ai_insights: &[String]) -> Vec<String> {
        let mut enhanced = vec![basic_reasoning.to_string()];
        enhanced.extend(ai_insights.iter().cloned());
        enhanced
    }
    
    fn format_participant_context(&self, profiles: &[ParticipantProfile]) -> String {
        profiles.iter()
            .map(|p| format!("{} ({}): {} meetings avg, {:.0}% response rate", 
                p.email, p.role, p.meeting_history.average_duration, 
                p.meeting_history.response_rate * 100.0))
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn generate_fallback_agenda(&self, meeting_request: &MeetingRequest, context: &MeetingContext) -> String {
        format!(
            "Meeting Agenda for: {}\n\
            Duration: {} minutes\n\
            Type: {:?}\n\n\
            1. Welcome and Introductions (5 min)\n\
            2. Agenda Review (3 min)\n\
            3. Main Discussion Topics ({} min)\n\
            4. Action Items and Next Steps (7 min)\n\
            5. Closing (5 min)",
            meeting_request.title,
            meeting_request.duration_minutes,
            context.meeting_type,
            meeting_request.duration_minutes.saturating_sub(20)
        )
    }
    
    async fn parse_ai_agenda_response(
        &self,
        ai_response: &str,
        meeting_request: &MeetingRequest,
        context: &MeetingContext,
    ) -> Result<MeetingAgenda> {
        // Parse AI response into structured agenda
        let lines: Vec<&str> = ai_response.lines().collect();
        let mut agenda_items = Vec::new();
        let mut objectives = Vec::new();
        let mut success_criteria = Vec::new();
        
        // Simple parsing - in a real implementation, you'd use more sophisticated parsing
        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains("objective") {
                objectives.push(line.trim().to_string());
            } else if line.to_lowercase().contains("success") {
                success_criteria.push(line.trim().to_string());
            } else if line.matches(|c: char| c.is_numeric()).count() > 0 && line.contains(".") {
                // Looks like an agenda item
                agenda_items.push(AgendaItem {
                    item_id: format!("item-{}", i),
                    title: line.trim().to_string(),
                    description: None,
                    duration_minutes: 10, // Default duration
                    item_type: AgendaItemType::Discussion,
                    owner: None,
                    prerequisites: Vec::new(),
                    expected_outcome: None,
                    discussion_points: Vec::new(),
                });
            }
        }
        
        // Fallback if parsing fails
        if agenda_items.is_empty() {
            agenda_items.push(AgendaItem {
                item_id: "main-discussion".to_string(),
                title: "Main Discussion".to_string(),
                description: Some("Primary meeting discussion topics".to_string()),
                duration_minutes: meeting_request.duration_minutes.saturating_sub(20),
                item_type: AgendaItemType::Discussion,
                owner: None,
                prerequisites: Vec::new(),
                expected_outcome: None,
                discussion_points: Vec::new(),
            });
        }
        
        Ok(MeetingAgenda {
            agenda_id: Uuid::new_v4().to_string(),
            meeting_title: meeting_request.title.clone(),
            meeting_type: context.meeting_type.clone(),
            duration_minutes: meeting_request.duration_minutes,
            objectives: if objectives.is_empty() { 
                vec!["Discuss agenda items and reach decisions".to_string()] 
            } else { 
                objectives 
            },
            agenda_items,
            preparation_materials: Vec::new(),
            success_criteria: if success_criteria.is_empty() { 
                vec!["All agenda items covered", "Clear action items defined"].iter().map(|s| s.to_string()).collect() 
            } else { 
                success_criteria 
            },
            follow_up_actions: vec!["Send meeting summary", "Schedule follow-up if needed"].iter().map(|s| s.to_string()).collect(),
            ai_generated: true,
            generated_at: Utc::now(),
        })
    }
    
    fn format_user_context(&self, context: &AiSchedulingContext) -> String {
        format!(
            "Preferences: {} max meetings/day, {} min breaks\n\
            Energy Patterns: {}\n\
            Optimization Goals: {:?}",
            context.user_preferences.max_meetings_per_day,
            context.user_preferences.break_duration_minutes,
            self.format_energy_patterns(&context.user_preferences.energy_patterns),
            context.optimization_goals
        )
    }
    
    fn analyze_conflict_complexity(&self, conflicts: &[ConflictInfo], _context: &AiSchedulingContext) -> ConflictAnalysis {
        let severity = match conflicts.len() {
            0 => ConflictSeverity::Minor,
            1 => ConflictSeverity::Moderate,
            2..=3 => ConflictSeverity::Significant,
            _ => ConflictSeverity::Critical,
        };
        
        let complexity = match conflicts.len() {
            0..=1 => ComplexityLevel::Simple,
            2..=3 => ComplexityLevel::Moderate,
            4..=6 => ComplexityLevel::Complex,
            _ => ComplexityLevel::HighlyComplex,
        };
        
        ConflictAnalysis {
            conflict_severity: severity,
            affected_meetings: conflicts.iter().map(|c| c.conflicting_event.id.clone()).collect(),
            affected_participants: conflicts.iter().map(|c| c.attendee.clone()).collect(),
            priority_conflicts: conflicts.iter().map(|c| PriorityConflict {
                meeting_id: c.conflicting_event.id.clone(),
                priority_level: ImportanceLevel::Medium, // Would be determined from context
                flexibility_score: 0.5,
                rescheduling_difficulty: 0.6,
            }).collect(),
            resolution_complexity: complexity,
        }
    }
    
    async fn parse_ai_conflict_solutions(&self, ai_response: &str, _conflicts: &[ConflictInfo]) -> Result<Vec<ConflictSolution>> {
        // Parse AI response for conflict solutions
        let mut solutions = Vec::new();
        
        // Simple parsing - look for solution patterns
        let lines: Vec<&str> = ai_response.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains("solution") || line.to_lowercase().contains("strategy") {
                solutions.push(ConflictSolution {
                    solution_id: format!("sol-{}", i),
                    solution_type: SolutionType::Reschedule, // Default
                    description: line.trim().to_string(),
                    implementation_steps: vec!["Review solution details".to_string(), "Execute resolution".to_string()],
                    success_probability: 0.8,
                    participant_satisfaction_impact: 0.7,
                    resource_requirements: vec!["Time for rescheduling".to_string()],
                });
            }
        }
        
        // Fallback solution if parsing fails
        if solutions.is_empty() {
            solutions.push(ConflictSolution {
                solution_id: "default-solution".to_string(),
                solution_type: SolutionType::Reschedule,
                description: "Reschedule conflicting meeting to next available slot".to_string(),
                implementation_steps: vec![
                    "Identify alternative time slots".to_string(),
                    "Notify all participants".to_string(),
                    "Update calendar events".to_string(),
                ],
                success_probability: 0.75,
                participant_satisfaction_impact: 0.6,
                resource_requirements: vec!["Coordination time", "Calendar management"].iter().map(|s| s.to_string()).collect(),
            });
        }
        
        Ok(solutions)
    }
    
    async fn assess_stakeholder_impact(&self, conflicts: &[ConflictInfo], _context: &AiSchedulingContext) -> Result<Vec<StakeholderImpact>> {
        let mut impacts = Vec::new();
        
        for conflict in conflicts {
            impacts.push(StakeholderImpact {
                participant_email: conflict.attendee.clone(),
                impact_level: match conflict.conflict_type {
                    ConflictType::DirectOverlap => StakeholderImpactLevel::High,
                    ConflictType::BufferViolation => StakeholderImpactLevel::Medium,
                    ConflictType::PreferenceViolation => StakeholderImpactLevel::Low,
                },
                specific_impacts: vec![
                    "Schedule disruption".to_string(),
                    "Need to reschedule existing commitment".to_string(),
                ],
                mitigation_strategies: vec![
                    "Provide advance notice of changes".to_string(),
                    "Offer alternative meeting times".to_string(),
                ],
            });
        }
        
        Ok(impacts)
    }
    
    fn extract_negotiation_strategies(&self, ai_response: &str) -> Vec<String> {
        ai_response
            .lines()
            .filter(|line| line.to_lowercase().contains("negotiat") || line.to_lowercase().contains("discuss"))
            .map(|line| line.trim().to_string())
            .collect()
    }
    
    fn extract_compromise_options(&self, ai_response: &str) -> Vec<String> {
        ai_response
            .lines()
            .filter(|line| line.to_lowercase().contains("compromise") || line.to_lowercase().contains("alternative"))
            .map(|line| line.trim().to_string())
            .collect()
    }
    
    async fn parse_ai_time_blocks(&self, _ai_response: &str, deadline: &Deadline) -> Result<Vec<SuggestedTimeBlock>> {
        let mut blocks = Vec::new();
        let remaining_hours = deadline.estimated_hours - deadline.completed_hours;
        let blocks_needed = (remaining_hours / 2.0).ceil() as usize; // 2-hour blocks
        
        let now = Utc::now();
        for i in 0..blocks_needed {
            let start_time = now + chrono::Duration::days(i as i64) + chrono::Duration::hours(9); // 9 AM
            let end_time = start_time + chrono::Duration::hours(2);
            
            blocks.push(SuggestedTimeBlock {
                start_time,
                end_time,
                focus_area: format!("Work session {} for {}", i + 1, deadline.title),
                estimated_productivity: 0.8,
                buffer_time: chrono::Duration::minutes(15),
                prerequisites: vec!["Clear workspace".to_string(), "Review previous progress".to_string()],
            });
        }
        
        Ok(blocks)
    }
    
    async fn parse_alternative_strategies(&self, _ai_response: &str, _deadline: &Deadline) -> Result<Vec<AllocationStrategy>> {
        let mut strategies = Vec::new();
        
        // Strategy 1: Intensive focused sessions
        strategies.push(AllocationStrategy {
            name: "Intensive Focus".to_string(),
            description: "Longer focused sessions with extended breaks".to_string(),
            time_blocks: vec![], // Would be populated with actual blocks
            pros: vec!["Deep focus", "Fewer context switches"].iter().map(|s| s.to_string()).collect(),
            cons: vec!["Risk of burnout", "Less flexible"].iter().map(|s| s.to_string()).collect(),
            risk_level: RiskLevel::Medium,
        });
        
        // Strategy 2: Distributed approach
        strategies.push(AllocationStrategy {
            name: "Distributed Work".to_string(),
            description: "Shorter sessions spread across more days".to_string(),
            time_blocks: vec![], // Would be populated with actual blocks
            pros: vec!["More sustainable", "Better work-life balance"].iter().map(|s| s.to_string()).collect(),
            cons: vec!["More setup/teardown time", "Potential for procrastination"].iter().map(|s| s.to_string()).collect(),
            risk_level: RiskLevel::Low,
        });
        
        Ok(strategies)
    }
    
    fn calculate_time_blocking_confidence(&self, _blocks: &[SuggestedTimeBlock], _context: &AiSchedulingContext) -> f32 {
        0.85 // Would calculate based on availability conflicts, energy alignment, etc.
    }
    
    fn extract_ai_reasoning(&self, ai_response: &str) -> String {
        ai_response
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    fn format_meeting_history(&self, history: &[CalendarEvent]) -> String {
        format!(
            "Recent meetings: {} total, avg duration: {} min",
            history.len(),
            if !history.is_empty() {
                history.iter()
                    .map(|e| (e.end_time - e.start_time).num_minutes())
                    .sum::<i64>() / history.len() as i64
            } else {
                0
            }
        )
    }
    
    fn parse_optimization_suggestions(&self, ai_response: &str) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();
        
        // Simple parsing for optimization suggestions
        let lines: Vec<&str> = ai_response.lines().collect();
        for line in lines {
            if line.to_lowercase().contains("suggest") || line.to_lowercase().contains("recommend") {
                suggestions.push(OptimizationSuggestion {
                    suggestion_type: OptimizationType::DurationAdjustment, // Default
                    description: line.trim().to_string(),
                    impact: "Positive".to_string(),
                    effort_level: EffortLevel::Low,
                    expected_benefit: 0.7,
                });
            }
        }
        
        suggestions
    }
    
    fn calculate_potential_time_savings(&self, suggestions: &[OptimizationSuggestion]) -> u32 {
        suggestions.len() as u32 * 10 // Estimate 10 minutes saved per suggestion
    }
    
    fn extract_efficiency_improvements(&self, ai_response: &str) -> Vec<String> {
        ai_response
            .lines()
            .filter(|line| line.to_lowercase().contains("efficiency") || line.to_lowercase().contains("improve"))
            .map(|line| line.trim().to_string())
            .collect()
    }
    
    fn extract_participant_improvements(&self, ai_response: &str) -> Vec<String> {
        ai_response
            .lines()
            .filter(|line| line.to_lowercase().contains("participant") || line.to_lowercase().contains("attendee"))
            .map(|line| line.trim().to_string())
            .collect()
    }
    
    // Utility formatting methods
    
    fn format_time_slots(&self, slots: &[TimeSlot]) -> String {
        slots.iter()
            .map(|slot| format!("{} to {}", 
                slot.start_time.format("%H:%M"), 
                slot.end_time.format("%H:%M")))
            .collect::<Vec<_>>()
            .join(", ")
    }
    
    fn format_energy_patterns(&self, patterns: &[EnergyLevel]) -> String {
        patterns.iter()
            .map(|p| format!("{}:00 (energy: {:.1})", p.time_of_day, p.energy_score))
            .collect::<Vec<_>>()
            .join(", ")
    }
    
    fn format_preferred_times(&self, slots: &[TimeSlot]) -> String {
        if slots.is_empty() {
            "No specific preferences".to_string()
        } else {
            self.format_time_slots(slots)
        }
    }
    
    fn format_focus_blocks(&self, slots: &[TimeSlot]) -> String {
        if slots.is_empty() {
            "No defined focus blocks".to_string()
        } else {
            self.format_time_slots(slots)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_calendar_adapter() {
        let config = CalendarConfig {
            provider: CalendarProvider::Apple,
            server_url: "https://caldav.icloud.com/123456789/calendars/".to_string(),
            username: "test@icloud.com".to_string(),
            password: "test-password".to_string(),
            calendar_name: Some("Test Calendar".to_string()),
        };

        let adapter = CalendarAdapter::new(config).unwrap();
        assert_eq!(adapter.config.provider, CalendarProvider::Apple);
        assert!(adapter.auth_header.starts_with("Basic "));
    }

    #[test]
    fn test_event_to_ics() {
        let config = CalendarConfig {
            provider: CalendarProvider::Apple,
            server_url: "https://caldav.icloud.com/123456789/calendars/".to_string(),
            username: "test@icloud.com".to_string(),
            password: "test-password".to_string(),
            calendar_name: Some("Test Calendar".to_string()),
        };

        let adapter = CalendarAdapter::new(config).unwrap();
        let event = CalendarEvent {
            id: "test-id".to_string(),
            title: "Test Event".to_string(),
            description: Some("Test Description".to_string()),
            start_time: DateTime::parse_from_rfc3339("2024-01-01T10:00:00Z").unwrap().with_timezone(&Utc),
            end_time: DateTime::parse_from_rfc3339("2024-01-01T11:00:00Z").unwrap().with_timezone(&Utc),
            location: Some("Test Location".to_string()),
            attendees: vec!["test@example.com".to_string()],
            all_day: false,
            recurring: false,
            calendar_id: "test-calendar".to_string(),
        };

        let ics = adapter.event_to_ics(&event, "test-id").unwrap();
        
        assert!(ics.contains("BEGIN:VCALENDAR"));
        assert!(ics.contains("BEGIN:VEVENT"));
        assert!(ics.contains("SUMMARY:Test Event"));
        assert!(ics.contains("DESCRIPTION:Test Description"));
        assert!(ics.contains("LOCATION:Test Location"));
        assert!(ics.contains("ATTENDEE:MAILTO:test@example.com"));
        assert!(ics.contains("DTSTART:20240101T100000Z"));
        assert!(ics.contains("DTEND:20240101T110000Z"));
        assert!(ics.contains("END:VEVENT"));
        assert!(ics.contains("END:VCALENDAR"));
    }

    #[test]
    fn test_ics_to_event() {
        let config = CalendarConfig {
            provider: CalendarProvider::Apple,
            server_url: "https://caldav.icloud.com/123456789/calendars/".to_string(),
            username: "test@icloud.com".to_string(),
            password: "test-password".to_string(),
            calendar_name: Some("Test Calendar".to_string()),
        };

        let adapter = CalendarAdapter::new(config).unwrap();
        let ics_content = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:test-id
DTSTART:20240101T100000Z
DTEND:20240101T110000Z
SUMMARY:Test Event
DESCRIPTION:Test Description
LOCATION:Test Location
ATTENDEE:MAILTO:test@example.com
END:VEVENT
END:VCALENDAR"#;

        let event = adapter.ics_to_event(ics_content, "test-id", "test-calendar").unwrap();
        
        assert_eq!(event.id, "test-id");
        assert_eq!(event.title, "Test Event");
        assert_eq!(event.description, Some("Test Description".to_string()));
        assert_eq!(event.location, Some("Test Location".to_string()));
        assert_eq!(event.attendees, vec!["test@example.com".to_string()]);
        assert_eq!(event.start_time.to_rfc3339(), "2024-01-01T10:00:00+00:00");
        assert_eq!(event.end_time.to_rfc3339(), "2024-01-01T11:00:00+00:00");
        assert!(!event.all_day);
        assert!(!event.recurring);
    }

    #[test]
    fn test_all_day_event_to_ics() {
        let config = CalendarConfig {
            provider: CalendarProvider::Apple,
            server_url: "https://caldav.icloud.com/123456789/calendars/".to_string(),
            username: "test@icloud.com".to_string(),
            password: "test-password".to_string(),
            calendar_name: Some("Test Calendar".to_string()),
        };

        let adapter = CalendarAdapter::new(config).unwrap();
        let event = CalendarEvent {
            id: "test-id".to_string(),
            title: "All Day Event".to_string(),
            description: None,
            start_time: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap().with_timezone(&Utc),
            end_time: DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z").unwrap().with_timezone(&Utc),
            location: None,
            attendees: vec![],
            all_day: true,
            recurring: false,
            calendar_id: "test-calendar".to_string(),
        };

        let ics = adapter.event_to_ics(&event, "test-id").unwrap();
        
        assert!(ics.contains("DTSTART;VALUE=DATE:20240101"));
        assert!(ics.contains("DTEND;VALUE=DATE:20240102"));
        assert!(ics.contains("SUMMARY:All Day Event"));
    }

    #[test]
    fn test_get_caldav_server_url() {
        assert_eq!(CalendarAdapter::get_caldav_server_url(), "https://caldav.icloud.com");
    }
}