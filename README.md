<img width="200" height="200" align="left" style="float: left; margin: 0 10px 0 0;" alt="Icon" src="https://github.com/tascord/ptvrs/blob/main/icon.png?raw=true"> 

# PTV (rs)
## Public transport Victoria's API in rust

[![GitHub top language](https://img.shields.io/github/languages/top/tascord/ptvrs?color=0072CE&style=for-the-badge)](#)
[![Crates.io Version](https://img.shields.io/crates/v/ptv?style=for-the-badge)](https://crates.io/crates/ptv)
[![docs.rs](https://img.shields.io/docsrs/ptv?style=for-the-badge)](https://docs.rs/ptv)

## Status
🟩 ; Complete, 🟦 ; To be tested ([you can help!](https://github.com/tascord/ptvrs/issues/new)), 🟨 ; Needs work, 🟥 ; Avoid use in current state ; ❌ Not implemented, yet.
| Feature       | Endpoint                                                                                                                                                                                 | Status | Notes |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----- |
| Runs          | [/v3/runs/{run_ref}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_runs_by_run_ref)                                                                                       | 🟩      |       |
|               | [/v3/runs/route/{route_id}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_runs_by_route_id)                                                                               | 🟩      |       |
|               | [/v3/runs/{run_ref}/route_type/{route_type}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_run_by_run_ref_and_route_type)                                                 | 🟩      |       |
|               | [/v3/runs/route/{route_id}/route_type/{route_type}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_runs_by_route_id_and_route_type)                                        | 🟩      |       |
| Outlets       | [/v3/outlets](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_outlets)                                                                                                      | 🟩      |       |
|               | [/v3/outlets/location/{latitude},{longitude}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_outlet_geolocation_by_latitude_and_longitude)                                 | 🟩      |       |
| Pattern       | [/v3/pattern/run/{run_ref}/route_type/{route_type}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_stopping_pattern_by_run_ref_and_route_type)                             | 🟩      |       |
| Stops         | [/v3/stops/{stop_id}/route_type/{route_type}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_stop_by_stop_id_and_route_type)                                               | 🟩      |       |
|               | [/v3/stops/location/{latitude},{longitude}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_stops_by_distance_by_latitude_and_longitude)                                    | 🟩      |       |
|               | [/v3/stops/route/{route_id}/route_type/{route_type}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_stops_on_route_by_route_id_and_route_type)                             | 🟩      |       |
| Search        | [/v3/search/{search_term}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_search_result_by_search_term)                                                                    | 🟩      |       |
| Routes        | [/v3/routes/{route_id}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_route_by_route_id)                                                                                  | 🟩      |       |
|               | [/v3/routes](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_routes)                                                                                                        | 🟩      |       |
| Route Types   | [/v3/route_types](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_route_types)                                                                                              | 🟩      |       |
| Departures    | [/v3/departures/route_type/{route_type}/stop/{stop_id}/route/{route_id}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_departures_by_route_type_and_stop_id_and_route_id) | 🟩      |       |
|               | [/v3/departures/route_type/{route_type}/stop/{stop_id}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_departures_by_route_type_and_stop_id)                               | 🟩      |       |
| Disruptions   | [/v3/disruptions/{disruption_id}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_disruption_by_disruption_id)                                                              | 🟩      |       |
|               | [/v3/disruptions/stop/{stop_id}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_disruptions_by_stop_id)                                                                    | 🟩      |       |
|               | [/v3/disruptions](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_disruptions)                                                                                              | 🟩      |       |
|               | [/v3/disruptions/route/{route_id}/stop/{stop_id}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_disruptions_by_route_id_and_stop_id)                                      | 🟩      |       |
|               | [/v3/disruptions/route/{route_id}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_disruptions_by_route_id)                                                                 | 🟩      |       |
| Directions    | [/v3/directions/{direction_id}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_directions_by_direction_id)                                                                 | 🟩      |       |
|               | [/v3/directions/{direction_id}/route_type/{route_type}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_directions_by_direction_id_and_route_type)                          | 🟩      |       |
|               | [/v3/directions/route/{route_id}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_directions_by_route_id)                                                                   | 🟩      |       |
| Fare Estimate | [/v3/fare_estimate/min_zone/{min_zone}/max_zone/{max_zone}](https://docs.rs/ptv/latest/ptv/struct.Client.html#method.get_fare_estimate_by_min_zone_and_max_zone)                         | 🟩      |       |
