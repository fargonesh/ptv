#![cfg(not(target_arch = "wasm32"))]
use {
    crate::*,
    anyhow::Result,
    hmac::{Hmac, Mac},
    serde::de::DeserializeOwned,
    sha1::Sha1,
};

pub const API_URL: &str = "https://timetableapi.ptv.vic.gov.au";

type PtvHmac = Hmac<Sha1>;

pub struct Client {
    devid: String,
    key: String,
}

impl Client {
    pub fn new(devid: String, key: String) -> Client {
        Client { devid, key }
    }

    pub async fn rq<T: DeserializeOwned>(&self, path: String) -> Result<T> {
        let path = format!(
            "/{path}{}devid={}",
            {
                if !path.contains('?') {
                    "?"
                } else if path.ends_with('?') {
                    ""
                } else {
                    "&"
                }
            },
            self.devid
        );

        let mut hasher: PtvHmac = Hmac::new_from_slice(self.key.as_bytes()).unwrap();
        hasher.update(path.as_bytes());

        let hash = hex::encode(hasher.finalize().into_bytes());
        let url = format!("{API_URL}{}&signature={}", path, hash);

        if std::env::var("DEBUG").is_ok() {
            println!("Requesting: {}", url);
        }

        let res = reqwest::get(&url).await?;
        if !res.status().is_success() {
            let status = res.status();
            if let Ok(ApiError { message, .. }) = res.json().await {
                return Err(anyhow::anyhow!("Request failed: {} - {}", status, message));
            }
            return Err(anyhow::anyhow!("Request failed: {}", status));
        }

        Ok(res.json().await?)
    }

    /* > Departures */

    /// View departures for all routes from a specific stop
    pub async fn departures_stop(
        &self,
        route_type: RouteType,
        stop_id: StopId,
        options: DeparturesStopOpts,
    ) -> Result<DeparturesResponse> {
        self.rq(format!(
            "v3/departures/route_type/{}/stop/{}?{}",
            route_type,
            stop_id,
            to_query(options)
        ))
        .await
    }

    /// View departures for a specific route from a stop
    pub async fn departures_stop_route(
        &self,
        route_type: RouteType,
        route_id: RouteId,
        stop_id: StopId,
        options: DeparturesStopRouteOpts,
    ) -> Result<DeparturesResponse> {
        self.rq(format!(
            "v3/departures/route_type/{}/stop/{}/route/{}?{}",
            route_type,
            route_id,
            stop_id,
            to_query(options)
        ))
        .await
    }

    /* > Directions */

    /// View all routes for a direction of travel
    pub async fn directions_id(&self, direction_id: DirectionId) -> Result<DirectionsResponse> {
        self.rq(format!("v3/directions/{}", direction_id)).await
    }

    /// View directions that a route travels in
    pub async fn directions_route(&self, route_id: RouteId) -> Result<DirectionsResponse> {
        self.rq(format!("v3/directions/route/{}", route_id)).await
    }

    /// View all routes of a particular type for a direction of travel
    pub async fn directions_id_route(
        &self,
        direction_id: DirectionId,
        route_type: RouteType,
    ) -> Result<DirectionsResponse> {
        self.rq(format!(
            "v3/directions/{}/route_type/{}",
            direction_id, route_type
        ))
        .await
    }

    /* > Disruptions */

    /// View all disruptions for all route types
    pub async fn disruptions(&self, options: DisruptionsOpts) -> Result<DisruptionsResponse> {
        self.rq(format!("v3/disruptions?{}", to_query(options)))
            .await
    }

    /// View all disruptions for a particular route
    pub async fn disruptions_route(
        &self,
        route_id: RouteId,
        options: DisruptionsSpecificOpts,
    ) -> Result<DisruptionsResponse> {
        self.rq(format!(
            "v3/disruptions/route/{}?{}",
            route_id,
            to_query(options)
        ))
        .await
    }

    /// View all disruptions for a particular route and stop
    pub async fn disruptions_route_stop(
        &self,
        route_id: RouteId,
        stop_id: StopId,
        options: DisruptionsSpecificOpts,
    ) -> Result<DisruptionsResponse> {
        self.rq(format!(
            "v3/disruptions/route/{}/stop/{}?{}",
            route_id,
            stop_id,
            to_query(options)
        ))
        .await
    }

    /// View all disruptions for a particular stop
    pub async fn disruptions_stop(
        &self,
        stop_id: StopId,
        options: DisruptionsSpecificOpts,
    ) -> Result<DisruptionsResponse> {
        self.rq(format!(
            "v3/disruptions/stop/{}?{}",
            stop_id,
            to_query(options)
        ))
        .await
    }

    /// View a specific disruption
    pub async fn disruptions_id(&self, disruption_id: DisruptionId) -> Result<Disruption> {
        // TODO: Technically this has Status too but I dont want to
        // dupe the struct 17 times
        self.rq(format!("v3/disruptions/{}", disruption_id)).await
    }

    /* > Fare Estimate */

    /// Estimate a fare by zone
    pub async fn fare_estimate(
        &self,
        min_zone: u8,
        max_zone: u8,
        options: FareEstimateOpts,
    ) -> Result<FareEstimateResponse> {
        self.rq(format!(
            "v3/fare_estimate/min_zone/{}/max_zone/{}?{}",
            min_zone,
            max_zone,
            to_query(options)
        ))
        .await
    }

    /* > Outlets */

    /// Last all ticket outlets
    pub async fn outlets(&self, options: OutletsOpts) -> Result<OutletsResponse> {
        self.rq(format!("v3/outlets?{}", to_query(options))).await
    }

    /// List outlets near a specific location
    pub async fn outlets_lat_long(
        &self,
        latitude: f64,
        longitude: f64,
        options: OutletsLatLongOpts,
    ) -> Result<OutletsResponse> {
        self.rq(format!(
            "v3/outlets/location/{}/{}?{}",
            latitude,
            longitude,
            to_query(options)
        ))
        .await
    }

    /* > Patterns */

    /// View the stopping pattern for a specific tip / service run
    pub async fn patterns_run_route(
        &self,
        run_ref: &str,
        route_type: RouteType,
        options: PatternsRunRouteOpts,
    ) -> Result<PatternResponse> {
        self.rq(format!(
            "v3/pattern/run/{}/route_type/{}?{}",
            run_ref,
            route_type,
            to_query(options)
        ))
        .await
    }

    /* > Routes */

    /// View route names and numbers for all routes
    pub async fn routes(&self, options: RouteOpts) -> Result<RoutesResponse> {
        self.rq(format!("v3/routes?{}", to_query(options))).await
    }

    // View route name and number for a specific route ID
    pub async fn routes_id(
        &self,
        route_id: RouteId,
        options: RouteIdOpts,
    ) -> Result<RoutesIdResponse> {
        self.rq(format!("v3/routes/{}?{}", route_id, to_query(options)))
            .await
    }

    /* > Runs */

    /// View all trip/service runs for a specific run_ref
    pub async fn runs_ref(&self, run_ref: &str, options: RunsRefOpts) -> Result<RunsResponse> {
        self.rq(format!("v3/runs/{}?{}", run_ref, to_query(options)))
            .await
    }

    /// View all trip/service runs for a specific route ID
    pub async fn runs_id(&self, run_id: RouteId, options: RunsIdOpts) -> Result<RunsResponse> {
        self.rq(format!("v3/runs/route/{}?{}", run_id, to_query(options)))
            .await
    }

    /// View all trip/service runs for a specific run_ref and route type
    pub async fn runs_ref_type(
        &self,
        run_ref: &str,
        route_type: RouteType,
        options: RunsRefOpts,
    ) -> Result<RunsResponse> {
        self.rq(format!(
            "v3/runs/{}/route_type/{}?{}",
            run_ref,
            route_type,
            to_query(options)
        ))
        .await
    }

    /// View all trip/service runs for a specific run ID and route type
    pub async fn runs_id_type(
        &self,
        run_id: RunId,
        route_type: RouteType,
        options: RunsIdOpts,
    ) -> Result<RunsResponse> {
        self.rq(format!(
            "v3/runs/route/{}/route_type/{}?{}",
            run_id,
            route_type,
            to_query(options)
        ))
        .await
    }
    // Search for stops, routes and myki outlets that match the input search term
    pub async fn search(&self, search_term: &str, options: SearchOpts) -> Result<SearchResponse> {
        self.rq(format!(
            "v3/search/{}?{}",
            url_escape::encode_path(&clean(search_term.to_owned())).into_owned(),
            to_query(options)
        ))
        .await
    }
    // View facilities at a specific stop (Metro and VLine stations only)
    pub async fn stops_id_route_type(
        &self,
        stop_id: StopId,
        route_type: RouteType,
        options: StopsIdRouteTypeOpts,
    ) -> Result<StopResponse> {
        self.rq(format!(
            "v3/stops/{}/route_type/{}?{}",
            stop_id,
            route_type,
            to_query(options)
        ))
        .await
    }
}
