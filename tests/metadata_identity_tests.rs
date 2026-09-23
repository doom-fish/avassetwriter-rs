use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use avassetwriter::{FileType, MetadataItem, MetadataValue, Writer};

const IDENTIFIER: &str = "mdta/com.apple.quicktime.title";
const LOADED_VALUE: &str = "authoritative value";

struct DropCounter(Arc<AtomicUsize>);

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct Artifact(PathBuf);

impl Drop for Artifact {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn lazy_aliases_filter_and_request_share_one_load() -> Result<(), Box<dyn std::error::Error>> {
    let loads = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = mpsc::sync_channel(1);
    let callback_loads = Arc::clone(&loads);
    let callback_drops = Arc::clone(&drops);
    let drop_counter = DropCounter(Arc::clone(&drops));
    let base = MetadataItem::string(IDENTIFIER, "stale seed");
    let lazy = MetadataItem::with_lazy_value_loader(&base, move |request| {
        let _keep_alive = &drop_counter;
        callback_loads.fetch_add(1, Ordering::SeqCst);
        let alias = request
            .metadata_item()
            .and_then(|item| {
                item.ok_or_else(|| {
                    avassetwriter::AVWriterError::InvalidState(
                        "value request omitted its metadata item".into(),
                    )
                })
            })
            .map_err(|error| error.to_string());
        let response = request
            .respond_with_value(&MetadataValue::String(LOADED_VALUE.into()))
            .map_err(|error| error.to_string());
        assert_eq!(callback_drops.load(Ordering::SeqCst), 0);
        let outcome = response.and(alias);
        let _ = sender.send(outcome);
    })?;

    let clone = lazy.clone();
    assert_eq!(loads.load(Ordering::SeqCst), 0);

    let filtered = MetadataItem::filtered_by_identifier(std::slice::from_ref(&clone), IDENTIFIER)?;
    assert_eq!(filtered.len(), 1);
    assert_eq!(loads.load(Ordering::SeqCst), 0);
    assert_eq!(
        filtered[0].value()?,
        MetadataValue::String(LOADED_VALUE.into())
    );
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 1);

    let request_alias = receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("lazy value request should complete")
        .expect("request metadata item should be retained");
    assert_eq!(request_alias.identifier(), IDENTIFIER);
    assert_eq!(
        request_alias.value()?,
        MetadataValue::String(LOADED_VALUE.into())
    );
    assert_eq!(lazy.value()?, MetadataValue::String(LOADED_VALUE.into()));
    assert_eq!(loads.load(Ordering::SeqCst), 1);

    drop(filtered);
    drop(clone);
    drop(lazy);
    drop(request_alias);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn property_builders_keep_loaded_value_and_chained_changes(
) -> Result<(), Box<dyn std::error::Error>> {
    let loads = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let callback_loads = Arc::clone(&loads);
    let drop_counter = DropCounter(Arc::clone(&drops));
    let base = MetadataItem::string(IDENTIFIER, "stale seed");
    let lazy = MetadataItem::with_lazy_value_loader(&base, move |request| {
        let _keep_alive = &drop_counter;
        callback_loads.fetch_add(1, Ordering::SeqCst);
        request
            .respond_with_value(&MetadataValue::String(LOADED_VALUE.into()))
            .expect("lazy metadata response should succeed");
    })?;

    assert_eq!(lazy.value()?, MetadataValue::String(LOADED_VALUE.into()));
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    let original_alias = lazy.clone();
    let built = lazy
        .with_data_type("com.example.metadata.text")
        .with_extended_language_tag("sv-SE")
        .with_locale_identifier("sv_SE");

    assert_eq!(loads.load(Ordering::SeqCst), 1);
    assert_eq!(built.data_type(), Some("com.example.metadata.text"));
    assert_eq!(built.extended_language_tag(), Some("sv-SE"));
    assert_eq!(built.locale_identifier(), Some("sv_SE"));
    assert_eq!(built.value()?, MetadataValue::String(LOADED_VALUE.into()));
    assert_eq!(
        original_alias.value()?,
        MetadataValue::String(LOADED_VALUE.into())
    );
    assert_eq!(loads.load(Ordering::SeqCst), 1);

    drop(original_alias);
    drop(built);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn writer_metadata_round_trip_preserves_lazy_identity() -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-artifacts");
    std::fs::create_dir_all(&artifacts)?;
    let path = artifacts.join(format!("metadata-identity-{}.mp4", std::process::id()));
    let _artifact = Artifact(path.clone());
    let writer = Writer::create(&path, FileType::Mp4)?;

    let loads = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let callback_loads = Arc::clone(&loads);
    let drop_counter = DropCounter(Arc::clone(&drops));
    let base = MetadataItem::string(IDENTIFIER, "stale seed");
    let lazy = MetadataItem::with_lazy_value_loader(&base, move |request| {
        let _keep_alive = &drop_counter;
        callback_loads.fetch_add(1, Ordering::SeqCst);
        request
            .respond_with_value(&MetadataValue::String(LOADED_VALUE.into()))
            .expect("lazy metadata response should succeed");
    })?;

    writer.set_metadata(std::slice::from_ref(&lazy))?;
    let round_trip = writer.metadata()?;
    assert_eq!(round_trip.len(), 1);
    assert_eq!(loads.load(Ordering::SeqCst), 0);
    assert_eq!(
        round_trip[0].value()?,
        MetadataValue::String(LOADED_VALUE.into())
    );
    assert_eq!(lazy.value()?, MetadataValue::String(LOADED_VALUE.into()));
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 1);

    drop(round_trip);
    drop(lazy);
    drop(writer);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}
