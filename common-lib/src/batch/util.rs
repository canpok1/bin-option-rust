use job_scheduler::{JobScheduler, Job};
use log::info;

use crate::error::MyResult;

pub fn start_scheduler<F>(cron_schedule: &str, f:F) -> MyResult<()> 
where
    F: Fn()
{
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
