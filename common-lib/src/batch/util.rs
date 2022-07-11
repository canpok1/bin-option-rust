use job_scheduler::{Job, JobScheduler};
use log::info;

use crate::error::MyResult;

pub fn start_scheduler<F>(cron_schedule: &str, f: F) -> MyResult<()>
where
    F: Fn(),
{
    if cron_schedule.is_empty() {
        info!("run onece only, cron schedule is empty");
        f();
        return Ok(());
    }

    let mut sched = JobScheduler::new();

    info!("set cron schedule: {}", cron_schedule);
    sched.add(Job::new(cron_schedule.parse()?, || {
        f();
    }));

    loop {
        sched.tick();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
