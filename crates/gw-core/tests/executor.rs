use gw_core::{
    cancel::Cancel,
    executor::{self, Event},
};
use std::{
    path::PathBuf,
    sync::{
        Barrier, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
#[test]
fn bounded_workers_really_overlap_and_keep_group_order() {
    let barrier = Barrier::new(3);
    let active = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let order = Mutex::new(Vec::new());
    let groups = (0..3)
        .map(|g| {
            vec![
                (g * 2, PathBuf::from(format!("{g}/0"))),
                (g * 2 + 1, PathBuf::from(format!("{g}/1"))),
            ]
        })
        .collect();
    let mut finished = Vec::new();
    executor::run(
        groups,
        3,
        &Cancel::default(),
        |path| {
            let n = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(n, Ordering::SeqCst);
            barrier.wait();
            order.lock().unwrap().push(path.to_owned());
            barrier.wait();
            active.fetch_sub(1, Ordering::SeqCst);
            path.to_owned()
        },
        |event| {
            if let Event::Finished { index, value } = event {
                finished.push((index, value))
            }
        },
    )
    .unwrap();
    assert_eq!(peak.load(Ordering::SeqCst), 3);
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert_eq!(finished.len(), 6);
    let order = order.into_inner().unwrap();
    for group in 0..3 {
        let first = PathBuf::from(format!("{group}/0"));
        let second = PathBuf::from(format!("{group}/1"));
        assert!(
            order.iter().position(|p| p == &first).unwrap()
                < order.iter().position(|p| p == &second).unwrap()
        );
    }
}
#[test]
fn cancel_stops_queued_work_and_joins_active_workers() {
    let cancel = Cancel::default();
    let started = AtomicUsize::new(0);
    let groups = (0..10)
        .map(|i| vec![(i, PathBuf::from(i.to_string()))])
        .collect();
    let result = executor::run(
        groups,
        1,
        &cancel,
        |_| {
            started.fetch_add(1, Ordering::SeqCst);
            cancel.cancel();
        },
        |_| {},
    );
    assert!(result.is_err());
    assert_eq!(started.load(Ordering::SeqCst), 1);
}
