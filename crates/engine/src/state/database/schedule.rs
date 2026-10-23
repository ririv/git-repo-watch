//! Transactional occurrence admission is independent of manual checks and reports.
use super::*;
impl Store {
    pub fn claim_schedule_occurrence(
        &self,
        plan: &str,
        slot: &str,
        cancel: &Cancel,
    ) -> Result<bool> {
        Database::write(self, cancel)?.transaction(cancel, |transaction| {
            Ok(transaction.execute("INSERT INTO scheduled_occurrences(plan,slot,claimed_at) VALUES(?1,?2,?3) ON CONFLICT(plan,slot) DO NOTHING", rusqlite::params![plan, slot, super::super::now()])? == 1)
        })
    }
}
