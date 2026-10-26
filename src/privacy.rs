//! Withholding identifying values from an otherwise complete snapshot.
//!
//! This runs after collection and before rendering, so the terminal text and
//! the JSON document cannot disagree about what was withheld. Redacted fields
//! keep their slot and their status; only the value is withheld.

use crate::model::*;

/// Redact the user name, the host name, the local addresses, and any field that
/// repeats one of them.
pub fn redact(snapshot: &mut SystemSnapshot) {
    withhold(&mut snapshot.identity.username);
    withhold(&mut snapshot.identity.hostname);
    // On platforms without a device-model source the Host row *is* the
    // hostname, so redacting the identity alone would leave it on screen.
    if snapshot
        .host
        .value()
        .is_some_and(|host| host.kind == HostKind::Hostname)
    {
        withhold(&mut snapshot.host);
    }
    withhold(&mut snapshot.local_ip);
    // The disk figures say nothing about the user, but the path they were read
    // from usually contains the user name. The counters stay.
    if let Detection::Available { value, .. } = &mut snapshot.disk
        && value.path != "/"
    {
        value.path = "~".to_owned();
    }
}

/// Withhold a value, leaving every other state alone. A deselected module stays
/// deselected and a failure stays a failure: rewriting either as "redacted"
/// would hide a fact the user needs — that nothing was collected, or that
/// something broke — behind one that is not true.
fn withhold<T>(field: &mut Detection<T>) {
    if matches!(field, Detection::Available { .. }) {
        *field = Detection::redacted();
    }
}
