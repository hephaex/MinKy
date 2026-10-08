#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::nitl::nitl_config::NitlConfig;

    #[tokio::test]
    async fn test_nitl_config_load_and_srcdoc() {
        // Test default config srcdoc
        let config = NitlConfig::default();
        let srcdoc = config.srcdoc();
        assert!(srcdoc.contains("version"));
        assert!(srcdoc.contains("rules"));
        assert!(srcdoc.contains("source_events"));
        assert!(srcdoc.contains("target_event"));

        // Create a test config with values
        let test_config = NitlConfig {
            version: 2,
            source_events: vec!["event1".to_string(), "event2".to_string()],
            target_event: Some("result_event".to_string()),
            ..Default::default()
        };

        let srcdoc = test_config.srcdoc();
        assert!(srcdoc.contains("version"));
        assert!(srcdoc.contains("2"));
        assert!(srcdoc.contains("event1"));
        assert!(srcdoc.contains("result_event"));
    }

    #[tokio::test]
    async fn test_nitl_service_creation() {
        let config = NitlConfig::default();
        let service = NitlService::new(config);
        assert_eq!(service.config().rules.len(), 0);

        // Test with custom config
        let custom_config = NitlConfig {
            version: 2,
            source_events: vec!["event1".to_string()],
            target_event: Some("result".to_string()),
            ..Default::default()
        };
        let service2 = NitlService::new(custom_config);
        assert_eq!(service2.config().version, 2);
        assert_eq!(service2.config().source_events, vec!["event1".to_string()]);
        assert_eq!(service2.config().target_event, Some("result".to_string()));
    }

    #[tokio::test]
    async fn test_nitl_service_async_methods() {
        let config = NitlConfig::default();
        let service = NitlService::new(config);

        // Test all async methods exist and can be called
        let test_name = "test_event";
        let result = service.run_test_event(test_name).await;
        assert!(result.is_ok());

        let event_name = "my_event";
        let publish_result = service.publish_event(event_name).await;
        assert!(publish_result.is_ok());

        let block_result = service.latest_block(event_name).await;
        assert!(block_result.is_ok());

        let test_sets_result = service.test_sets_for_event(event_name).await;
        assert!(test_sets_result.is_ok());

        // Test empty string validation
        let empty_result = service.run_test_event("").await;
        assert!(empty_result.is_err());
    }

    #[tokio::test]
    async fn test_nitl_serialization() {
        let original_config = NitlConfig {
            version: 2,
            source_events: vec!["event1".to_string(), "event2".to_string()],
            target_event: Some("result".to_string()),
            rules: vec![
                NitlRule {
                    focus_id: "rule1".to_string(),
                    dependencies: vec!["dep1".to_string()],
                    ..Default::default()
                }
            ],
        };

        // Serialize
        let json = serde_json::to_string(&original_config).unwrap();
        assert!(json.contains("2"));
        assert!(json.contains("event1"));

        // Deserialize
        let deserialized: NitlConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, original_config.version);
        assert_eq!(deserialized.source_events, original_config.source_events);
        assert_eq!(deserialized.target_event, original_config.target_event);
        assert_eq!(deserialized.rules.len(), 1);
        assert_eq!(deserialized.rules[0].focus_id, "rule1");
    }

    #[tokio::test]
    async fn test_nitl_integration_full_flow() {
        // Create config with multiple source events
        let mut config = NitlConfig::default();
        config.version = 1;
        config.source_events = vec!["commit".to_string(), "sync".to_string()];
        config.target_event = Some("notify".to_string());

        // Create service
        let service = NitlService::new(config);

        // Verify config is accessible
        assert_eq!(service.config().version, 1);
        assert_eq!(service.config().source_events.len(), 2);
        assert_eq!(service.config().target_event, Some("notify".to_string()));

        // Test async methods
        let test_event = "integration_test";
        let _ = service.run_test_event(test_event).await;
        let _ = service.publish_event(test_event).await;
        let _ = service.latest_block(test_event).await;
        let _ = service.test_sets_for_event(test_event).await;
    }
}
