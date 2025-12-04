#[allow(dead_code)]
#[cfg(test)]
pub mod test {
    use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

    use colored::Colorize;
    use dotenv::dotenv;
    use futures::{StreamExt, stream::FuturesUnordered};

    use once_cell::sync::Lazy;
    use ptv::core::generated_types::*;
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
    static NOW: Lazy<chrono::NaiveDateTime> = Lazy::new(|| chrono::Utc::now().naive_utc());

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
        println!(
            "{}",
            serde_json::to_string_pretty(&DisruptionMode::MetroTrain).unwrap()
        );
        make_test!(map, get_departures_by_route_type_and_stop_id, GetDeparturesByRouteTypeAndStopIdParams => [ gtfs, include_cancelled, date_utc: DateTime::Naive(*NOW)], ROUTE_TYPE, STOP_ID );
        make_test!(map, get_departures_by_route_type_and_stop_id_and_route_id, GetDeparturesByRouteTypeAndStopIdAndRouteIdParams => [ gtfs, include_cancelled], ROUTE_TYPE, STOP_ID, ROUTE_ID );
        make_test!(map, get_directions_by_direction_id, DIRECTION_ID);
        make_test!(
            map,
            get_directions_by_direction_id_and_route_type,
            DIRECTION_ID,
            ROUTE_TYPE
        );
        make_test!(map, get_directions_by_route_id, ROUTE_ID);
        make_test!(map, get_disruption_by_disruption_id, DisruptionId(1));
        make_test!(map, get_disruptions, GetDisruptionsParams => [ route_types: vec![RouteType::Train], disruption_status: DisruptionStatus::Planned, disruption_status: DisruptionStatus::Current, disruption_modes: Modes(DisruptionMode::MetroTrain) ]);
        make_test!(map, get_disruptions_by_route_id, GetDisruptionsByRouteIdParams => [ disruption_status: DisruptionStatus::Planned, disruption_status: DisruptionStatus::Current ], ROUTE_ID);
        make_test!(map, get_disruptions_by_route_id_and_stop_id, GetDisruptionsByRouteIdAndStopIdParams => [ disruption_status: DisruptionStatus::Planned ], ROUTE_ID, STOP_ID);
        make_test!(map, get_disruptions_by_stop_id, GetDisruptionsByStopIdParams => [ disruption_status: DisruptionStatus::Planned ], STOP_ID);
        make_test!(map, get_fare_estimate_by_min_zone_and_max_zone,GetFareEstimateByMinZoneAndMaxZoneParams => [ is_journey_in_free_tram_zone ], 1, 2);
        make_test!(map, get_outlet_geolocation_by_latitude_and_longitude, GetOutletGeolocationByLatitudeAndLongitudeParams => [ max_results: 20, max_distance: 30.0 ], -37.8100, 144.9620);
        make_test!(map, get_outlets, GetOutletsParams => [ max_results: 20 ]);
        make_test!(map, get_route_by_route_id, GetRouteByRouteIdParams => [include_geopath], ROUTE_ID);
        make_test!(map, get_routes, GetRoutesParams => [ route_types: vec![RouteType::Train], route_types: vec![RouteType::Bus], route_types: vec![RouteType::Tram] ]);
        make_test!(map, get_run_by_run_ref_and_route_type, GetRunByRunRefAndRouteTypeParams => [expand: vec![ExpandOptions::All]], RUN_REF, ROUTE_TYPE);
        make_test!(map, get_runs_by_route_id, GetRunsByRouteIdParams => [ expand: vec![ExpandOptions::Direction] ], ROUTE_ID);
        make_test!(map, get_runs_by_route_id_and_route_type, GetRunsByRouteIdAndRouteTypeParams => [ expand: vec![ExpandOptions::Direction] ], ROUTE_ID, ROUTE_TYPE);
        make_test!(map, get_runs_by_run_ref, GetRunsByRunRefParams => [ expand: vec![ExpandOptions::All] ], RUN_REF);
        make_test!(map, get_search_result_by_search_term, GetSearchResultBySearchTermParams => [ include_addresses, route_types: vec![RouteType::Train], route_types: vec![RouteType::Bus] ], "Flinders Street");
        make_test!(map, get_stop_by_stop_id_and_route_type, GetStopByStopIdAndRouteTypeParams => [ stop_location, stop_amenities  ], STOP_ID, ROUTE_TYPE);
        make_test!(map, get_stopping_pattern_by_run_ref_and_route_type, GetStoppingPatternByRunRefAndRouteTypeParams => [ expand: vec![ExpandOptions::All] ], RUN_REF, ROUTE_TYPE);
        make_test!(map, get_stops_by_distance_by_latitude_and_longitude, GetStopsByDistanceByLatitudeAndLongitudeParams => [ max_results: 20, max_distance: 30.0 ], -37.8100, 144.9620);
        make_test!(map, get_stops_on_route_by_route_id_and_route_type, GetStopsOnRouteByRouteIdAndRouteTypeParams => [ include_geopath ], ROUTE_ID, ROUTE_TYPE);
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
