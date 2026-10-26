//! Withholding identifying values from an otherwise complete snapshot.
//!
//! This runs after collection and before rendering, so the terminal text and
//! the JSON document cannot disagree about what was withheld. Redacted fields
//! keep their slot and their status; only the value is withheld.

use crate::model::*;

/// Redact the user name, the host name, the local addresses, and any field that
/// repeats one of them.
pub fn redact(snapshot: &mut SystemSnapshot) {
    snapshot.identity.username = Detection::redacted();
    snapshot.identity.hostname = Detection::redacted();
    // On platforms without a device-model source the Host row *is* the
    // hostname, so redacting the identity alone would leave it on screen.
    if snapshot
        .host
        .value()
        .is_some_and(|host| host.kind == HostKind::Hostname)
    {
        snapshot.host = Detection::redacted();
    }
    snapshot.local_ip = Detection::redacted();
    // The disk figures say nothing about the user, but the path they were read
    // from usually contains the user name. The counters stay.
    if let Detection::Available { value, .. } = &mut snapshot.disk
        && value.path != "/"
    {
        value.path = "~".to_owned();
    }
}
