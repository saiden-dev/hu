use super::*;

#[test]
fn incident_status_deserialize() {
    let json = r#""triggered""#;
    let status: IncidentStatus = serde_json::from_str(json).unwrap();
    assert_eq!(status, IncidentStatus::Triggered);

    let json = r#""acknowledged""#;
    let status: IncidentStatus = serde_json::from_str(json).unwrap();
    assert_eq!(status, IncidentStatus::Acknowledged);

    let json = r#""resolved""#;
    let status: IncidentStatus = serde_json::from_str(json).unwrap();
    assert_eq!(status, IncidentStatus::Resolved);
}

#[test]
fn incident_status_serialize() {
    let json = serde_json::to_string(&IncidentStatus::Triggered).unwrap();
    assert_eq!(json, r#""triggered""#);

    let json = serde_json::to_string(&IncidentStatus::Acknowledged).unwrap();
    assert_eq!(json, r#""acknowledged""#);

    let json = serde_json::to_string(&IncidentStatus::Resolved).unwrap();
    assert_eq!(json, r#""resolved""#);
}

#[test]
fn incident_status_as_str() {
    assert_eq!(IncidentStatus::Triggered.as_str(), "triggered");
    assert_eq!(IncidentStatus::Acknowledged.as_str(), "acknowledged");
    assert_eq!(IncidentStatus::Resolved.as_str(), "resolved");
}

#[test]
fn urgency_deserialize() {
    let json = r#""high""#;
    let urgency: Urgency = serde_json::from_str(json).unwrap();
    assert_eq!(urgency, Urgency::High);

    let json = r#""low""#;
    let urgency: Urgency = serde_json::from_str(json).unwrap();
    assert_eq!(urgency, Urgency::Low);
}

#[test]
fn urgency_serialize() {
    let json = serde_json::to_string(&Urgency::High).unwrap();
    assert_eq!(json, r#""high""#);

    let json = serde_json::to_string(&Urgency::Low).unwrap();
    assert_eq!(json, r#""low""#);
}

#[test]
fn user_deserialize() {
    let json = r#"{
            "id": "U123",
            "name": "Alice Smith",
            "email": "alice@example.com",
            "html_url": "https://pagerduty.com/users/U123"
        }"#;
    let user: User = serde_json::from_str(json).unwrap();
    assert_eq!(user.id, "U123");
    assert_eq!(user.display_name(), "Alice Smith");
    assert_eq!(user.email, "alice@example.com");
    assert_eq!(user.html_url, "https://pagerduty.com/users/U123");
}

#[test]
fn user_deserialize_without_html_url() {
    let json = r#"{
            "id": "U123",
            "name": "Alice Smith",
            "email": "alice@example.com"
        }"#;
    let user: User = serde_json::from_str(json).unwrap();
    assert_eq!(user.html_url, "");
}

#[test]
fn oncall_deserialize() {
    let json = r#"{
            "user": {"id": "U1", "name": "Alice", "email": "alice@example.com"},
            "escalation_policy": {"id": "EP1", "name": "Primary"},
            "escalation_level": 1,
            "schedule": null,
            "start": "2026-01-01T00:00:00Z",
            "end": "2026-01-08T00:00:00Z"
        }"#;
    let oncall: Oncall = serde_json::from_str(json).unwrap();
    assert_eq!(oncall.user.display_name(), "Alice");
    assert_eq!(oncall.escalation_level, 1);
    assert!(oncall.schedule.is_none());
    assert_eq!(oncall.start, Some("2026-01-01T00:00:00Z".to_string()));
}

#[test]
fn oncall_deserialize_with_schedule() {
    let json = r#"{
            "user": {"id": "U1", "name": "Alice", "email": "alice@example.com"},
            "escalation_policy": {"id": "EP1", "name": "Primary"},
            "escalation_level": 2,
            "schedule": {"id": "S1", "name": "Weekly Rotation"},
            "start": null,
            "end": null
        }"#;
    let oncall: Oncall = serde_json::from_str(json).unwrap();
    assert!(oncall.schedule.is_some());
    assert_eq!(oncall.schedule.unwrap().name, "Weekly Rotation");
    assert_eq!(oncall.escalation_level, 2);
}

#[test]
fn incident_deserialize() {
    let json = r#"{
            "id": "INC123",
            "incident_number": 42,
            "title": "Server down",
            "status": "triggered",
            "urgency": "high",
            "created_at": "2026-01-01T12:00:00Z",
            "html_url": "https://pagerduty.com/incidents/INC123",
            "service": {"id": "S1", "name": "Production", "status": "active"},
            "assignments": []
        }"#;
    let incident: Incident = serde_json::from_str(json).unwrap();
    assert_eq!(incident.id, "INC123");
    assert_eq!(incident.incident_number, 42);
    assert_eq!(incident.status, IncidentStatus::Triggered);
    assert_eq!(incident.urgency, Urgency::High);
    assert_eq!(incident.service.name, "Production");
}

#[test]
fn incident_deserialize_with_assignments() {
    let json = r#"{
            "id": "INC123",
            "incident_number": 42,
            "title": "Server down",
            "status": "acknowledged",
            "urgency": "low",
            "created_at": "2026-01-01T12:00:00Z",
            "service": {"id": "S1", "name": "Production", "status": "active"},
            "assignments": [
                {"assignee": {"id": "U1", "name": "Alice", "email": "alice@example.com"}}
            ]
        }"#;
    let incident: Incident = serde_json::from_str(json).unwrap();
    assert_eq!(incident.assignments.len(), 1);
    assert_eq!(incident.assignments[0].assignee.display_name(), "Alice");
}

#[test]
fn oncalls_response_deserialize() {
    let json = r#"{"oncalls": []}"#;
    let resp: OncallsResponse = serde_json::from_str(json).unwrap();
    assert!(resp.oncalls.is_empty());
}

#[test]
fn incidents_response_deserialize() {
    let json = r#"{"incidents": []}"#;
    let resp: IncidentsResponse = serde_json::from_str(json).unwrap();
    assert!(resp.incidents.is_empty());
}

#[test]
fn services_response_deserialize() {
    let json = r#"{"services": []}"#;
    let resp: ServicesResponse = serde_json::from_str(json).unwrap();
    assert!(resp.services.is_empty());
}

#[test]
fn current_user_response_deserialize() {
    let json = r#"{"user": {"id": "U1", "name": "Alice", "email": "alice@example.com"}}"#;
    let resp: CurrentUserResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.user.display_name(), "Alice");
}

#[test]
fn output_format_default() {
    let format = OutputFormat::default();
    assert_eq!(format, OutputFormat::Table);
}

#[test]
fn output_format_eq() {
    assert_eq!(OutputFormat::Table, OutputFormat::Table);
    assert_eq!(OutputFormat::Json, OutputFormat::Json);
    assert_ne!(OutputFormat::Table, OutputFormat::Json);
}

#[test]
fn types_are_debug() {
    // Ensure all types implement Debug
    let user = User {
        id: "U1".to_string(),
        name: Some("Alice".to_string()),
        summary: None,
        email: "alice@example.com".to_string(),
        html_url: String::new(),
    };
    let _ = format!("{:?}", user);
    let _ = format!("{:?}", IncidentStatus::Triggered);
    let _ = format!("{:?}", Urgency::High);
    let _ = format!("{:?}", OutputFormat::Table);
}

#[test]
fn types_are_clone() {
    let user = User {
        id: "U1".to_string(),
        name: Some("Alice".to_string()),
        summary: None,
        email: "alice@example.com".to_string(),
        html_url: String::new(),
    };
    let cloned = user.clone();
    assert_eq!(cloned.id, user.id);

    let status = IncidentStatus::Triggered;
    let cloned = status;
    assert_eq!(cloned, status);
}

#[test]
fn user_display_name_prefers_name() {
    let user = User {
        id: "U1".to_string(),
        name: Some("Alice".to_string()),
        summary: Some("Alice Summary".to_string()),
        email: String::new(),
        html_url: String::new(),
    };
    assert_eq!(user.display_name(), "Alice");
}

#[test]
fn user_display_name_falls_back_to_summary() {
    let user = User {
        id: "U1".to_string(),
        name: None,
        summary: Some("Alice Summary".to_string()),
        email: String::new(),
        html_url: String::new(),
    };
    assert_eq!(user.display_name(), "Alice Summary");
}

#[test]
fn user_display_name_falls_back_to_id() {
    let user = User {
        id: "U1".to_string(),
        name: None,
        summary: None,
        email: String::new(),
        html_url: String::new(),
    };
    assert_eq!(user.display_name(), "U1");
}
