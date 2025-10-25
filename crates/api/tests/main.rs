#[allow(dead_code)]
#[cfg(test)]
pub mod test {
    use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

    use colored::Colorize;
    use dotenv::dotenv;
    use futures::{StreamExt, stream::FuturesUnordered};

    use once_cell::sync::Lazy;
    use ptv::*;
    use ptvrs_macros::make_test;

    static DEVID: &str = "DEVID";
    static KEY: &str = "KEY";

    static CLIENT: Lazy<Client> = Lazy::new(|| {
        // Load .env file if DEVID and KEY are not set
        if let (Ok(devid), Ok(key)) = (std::env::var(DEVID), std::env::var(KEY)) {
            Client::new(devid, key)
        } else {
            dotenv().ok();
            Client::new(std::env::var(DEVID).unwrap(), std::env::var(KEY).unwrap())
        }
    });

    // TODO: Find sensible constants
    static ROUTE_TYPE: RouteType = RouteType::Train; // Train
    static ROUTE_ID: RouteId = RouteId(1); // Alamein (Line)
    static STOP_ID: StopId = StopId(1002); // Alamein (Station)
    static DIRECTION_ID: DirectionId = DirectionId(1); // Towards Flinders Street
    static RUN_REF: &str = "1"; // Alamein something

    type Task =
        Arc<dyn Fn() -> Pin<Box<dyn Future<Output = anyhow::Result<String>>>> + Send + Sync>;
    pub static TASKS: Lazy<BTreeMap<&str, Task>> = Lazy::new(|| {
        let mut map = BTreeMap::<&str, Task>::new();

        // > Departures
        make_test!(
            map,
            departures_stop,
            DeparturesStopOpts => [gtfs,include_cancelled],
            ROUTE_TYPE,
            STOP_ID
        );

        make_test!(
            map,
            departures_stop_route,
            DeparturesStopRouteOpts => [[max_results: 10, look_backwards: true],gtfs,include_cancelled],
            ROUTE_TYPE,
            ROUTE_ID,
            STOP_ID
        );
        // > Routes
        make_test!(
            map,
            routes,
            RouteOpts => [route_types: vec![RouteType::Train]]
        );

        make_test!(map, routes_id,  RouteIdOpts => [include_geopath], ROUTE_ID);

        // > Patterns
        make_test!(map, patterns_run_route, PatternsRunRouteOpts => [stop_id: STOP_ID, expand: vec![ExpandOptions::All], include_skipped, include_geopath], RUN_REF, ROUTE_TYPE);

        // > Directions

        make_test!(map, directions_id, DIRECTION_ID);

        make_test!(map, directions_route, ROUTE_ID);

        make_test!(map, directions_id_route, DIRECTION_ID, ROUTE_TYPE);

        // > Disruptions

        make_test!(map, disruptions, DisruptionsOpts => [modes: vec![DisruptionModes::MetroTrain], modes: vec![DisruptionModes::MetroBus]]);

        make_test!(
            map,
            disruptions_route,
            DisruptionsSpecificOpts => [
                status: DisruptionStatus::Current,
                status: DisruptionStatus::Planned
            ],
            ROUTE_ID
        );

        make_test!(
            map,
            disruptions_route_stop,
            DisruptionsSpecificOpts => [
                status: DisruptionStatus::Current,
                status: DisruptionStatus::Planned
            ],
            ROUTE_ID,
            STOP_ID
        );

        make_test!(
            map,
            disruptions_stop,
            DisruptionsSpecificOpts => [
                status: DisruptionStatus::Current,
                status: DisruptionStatus::Planned
            ],
            STOP_ID
        );

        // > Search
        make_test!(map, search, SearchOpts => [include_outlets, include_addresses],"Flinders Street Station");

        map
    });

    //

    #[test]
    pub fn test() {
        let failed = Arc::new(tokio::sync::Mutex::new(0usize));

        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .max_blocking_threads(4)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut tasks = TASKS
                    .iter()
                    .map(|(name, task)| {
                        let task = Arc::clone(task);
                        let failed = Arc::clone(&failed);
                        async move {
                            println!("[{}] Running test: {}", "~".cyan(), name.yellow());
                            let start = std::time::Instant::now();
                            let res = task().await;
                            let elapsed = start.elapsed();
                            match res {
                                Ok(res) => println!(
                                    "[{}] {} {} in {:?}:{}",
                                    "+".green(),
                                    name.yellow(),
                                    "passed".green(),
                                    elapsed,
                                    {
                                        if std::env::var("QUIET").is_err() {
                                            format!("\n{}", res.cyan())
                                        } else {
                                            " ...".cyan().to_string()
                                        }
                                    }
                                ),
                                Err(e) => {
                                    {
                                        let mut failed = failed.lock().await;
                                        *failed += 1;
                                    }
                                    println!(
                                        "[{}] {} {} in {:?}:\n{}",
                                        "-".red(),
                                        name.yellow(),
                                        "failed".red(),
                                        elapsed,
                                        e.to_string().cyan()
                                    )
                                }
                            }
                        }
                    })
                    .collect::<FuturesUnordered<_>>();
                while (tasks.next().await).is_some() {}
            });

        let failed = failed.blocking_lock();
        if *failed > 0 {
            panic!("{} tests failed", failed);
        }

        println!("\n{}", "All tests passed! :3".green());
    }
}
