use super::fixture::*;

keyringd_testkit::scenario! {
    group "Profile persistence format" {
        case "Stores and restores each key with its own settings in schema version one" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            given "a profile with a Device Key handle and a complete Passkey credential" {
                let record = persisted_profile(ProfileId::new(), "Stored", DevicePrivateKeyHandle::new(b"stored-key".to_vec())).await;
                let document = ProfilesDocument { schema_version: PROFILES_SCHEMA_VERSION, profiles: vec![record] };
            }
            when "the document is serialized to JSON and deserialized for startup" {
                let bytes = document.to_json().unwrap();
                let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                let restored = ProfilesDocument::from_json(&bytes).unwrap();
                let application = KeyringdApplication::new(
                    crate::test_support::successful_device_keys(),
                    TestPasskeyService::new(),
                    Arc::new(MemoryRepository::with_state(restored)),

                ).unwrap();
            }
            then "metadata is separate from the camelCase key records and startup preserves the key policies" {
                assert_eq!(json["schemaVersion"], 1);
                let profile = json["profiles"][0].as_object().unwrap();
                assert_eq!(profile.keys().map(String::as_str).collect::<Vec<_>>(), ["deviceKey", "displayName", "enabled", "id", "passkey"]);
                assert_eq!(profile["deviceKey"].as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(), ["certificate", "handle", "localAuthentication", "signatureAlgorithm"]);
                assert_eq!(profile["passkey"].as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(), ["attestationResponse", "credentialId", "publicKey", "signatureAlgorithm", "userHandle"]);
                assert_eq!(profile["deviceKey"]["signatureAlgorithm"], "ecdsa-sha2-nistp256");
                assert_eq!(profile["passkey"]["signatureAlgorithm"], "sk-ecdsa-sha2-nistp256@openssh.com");
                let restored = &application.profiles()[0];
                assert_eq!(serde_json::to_value(restored).unwrap(), serde_json::to_value(&document.profiles[0]).unwrap());
            }
        }
        case "Rejects obsolete profile fields and snake_case credential fields instead of accepting aliases" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            given "a schema version one document" {
                let record = persisted_profile(ProfileId::new(), "Stored", DevicePrivateKeyHandle::new(b"stored-key".to_vec())).await;
                let document = ProfilesDocument { schema_version: PROFILES_SCHEMA_VERSION, profiles: vec![record] };
                let json = serde_json::to_value(&document).unwrap();
            }
            when "legacy top-level and credential field names are supplied" {
                let mut obsolete = json.clone();
                obsolete["profiles"][0]["deviceKeyUserVerification"] = serde_json::json!("notRequired");
                let obsolete_result = serde_json::from_value::<ProfilesDocument>(obsolete);
                let mut snake_case = json;
                let passkey = snake_case["profiles"][0]["passkey"].as_object_mut().unwrap();
                let credential = passkey.remove("credentialId").unwrap();
                passkey.insert("credential_id".into(), credential);
                let snake_case_result = serde_json::from_value::<ProfilesDocument>(snake_case);
            }
            then "both obsolete forms fail deserialization" {
                assert!(obsolete_result.is_err());
                assert!(snake_case_result.is_err());
            }
        }
        case "Rejects invalid profile data while decoding JSON" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "invalid stored fields and duplicate profile IDs are rejected during decoding" {
                let profile = persisted_profile(
                    ProfileId::new(),
                    "Stored",
                    DevicePrivateKeyHandle::new(b"stored".to_vec()),
                )
                .await;
                let document = ProfilesDocument {
                    profiles: vec![profile.clone()],
                    ..ProfilesDocument::default()
                };
                let valid = serde_json::to_value(&document).unwrap();
                for (path, replacement, reason) in [
                    ("/profiles/0/id", serde_json::json!("bad id"), "invalid ID"),
                    (
                        "/profiles/0/displayName",
                        serde_json::json!(" Stored "),
                        "profile name",
                    ),
                    (
                        "/profiles/0/displayName",
                        serde_json::json!(""),
                        "profile name",
                    ),
                    (
                        "/profiles/0/displayName",
                        serde_json::json!("é".repeat(81)),
                        "profile name",
                    ),
                    (
                        "/profiles/0/passkey/userHandle",
                        serde_json::json!(""),
                        "user handle",
                    ),
                    (
                        "/profiles/0/passkey/publicKey",
                        serde_json::json!("AA"),
                        "public key",
                    ),
                    (
                        "/profiles/0/passkey/signatureAlgorithm",
                        serde_json::json!("ecdsa-sha2-nistp256"),
                        "signature algorithm",
                    ),
                    (
                        "/profiles/0/deviceKey/certificate",
                        serde_json::json!("AA"),
                        "",
                    ),
                    (
                        "/profiles/0/deviceKey/certificate",
                        serde_json::json!("%%%"),
                        "",
                    ),
                ] {
                    let mut invalid = valid.clone();
                    *invalid.pointer_mut(path).unwrap() = replacement;
                    let error = ProfilesDocument::from_json(&serde_json::to_vec(&invalid).unwrap())
                        .err()
                        .expect("invalid profile must be rejected during decoding");
                    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
                    if !reason.is_empty() {
                        assert!(error.to_string().contains(reason), "{path}: {error}");
                    }
                }
                let duplicates = ProfilesDocument {
                    profiles: vec![profile.clone(), profile],
                    ..ProfilesDocument::default()
                };
                let error = ProfilesDocument::from_json(&serde_json::to_vec(&duplicates).unwrap())
                    .err()
                    .expect("duplicate IDs must be rejected");
                assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
                assert!(error.to_string().contains("duplicate profile ID"));
            }
        }
    }
}
