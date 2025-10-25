//! Some types are marked as 'Value' and have a note, TODO: T.
//! This is because the API documentation does not specify the type of the value.
//! I'll do my best to fill in what appears to be correct, but it's not guaranteed to be correct.
//!
//! I appreciate any work done to fill in the TODO: T types.

use chrono::{NaiveDate, NaiveDateTime};
use derive_more::{Display, From};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, str::FromStr};
use to_and_fro::{output_case, ToAndFro};

use crate::helpers::{
    de_iso_8601, de_rfc3339, de_service_time, deserialize_path, opt_de_rfc3339, opt_ser_rfc3339,
    ser_disruption_query, ser_iso_8601, ser_rfc3339, ser_touch_utc,
};

pub struct I32ButSilly(pub i32);
impl<'de> Deserialize<'de> for I32ButSilly {
    fn deserialize<D>(deserializer: D) -> Result<I32ButSilly, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(I32ButSilly(
            i32::from_str(&value).map_err(|e| serde::de::Error::custom(format!("{e:?}")))?,
        ))
    }
}

macro_rules! newtype_i32 {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, Deserialize, Serialize, Display, PartialEq, Eq, PartialOrd, Ord)]
        #[serde(transparent)]
        pub struct $name(pub i32);
    };
    ($name:ident, $($extra:tt)*) => {
        #[derive(Debug, Copy, Clone, Deserialize, Serialize, Display, PartialEq, Eq, PartialOrd, Ord, $($extra)*)]
        #[serde(transparent)]
        pub struct $name(pub i32);
    };
}
newtype_i32!(DisruptionId);

newtype_i32!(RunId);

newtype_i32!(StopId);

newtype_i32!(RouteId);

newtype_i32!(DirectionId);

/// Routepath (TODO)
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Geopath {
    pub direction_id: DirectionId,
    pub valid_from: NaiveDate,
    pub valid_to: NaiveDate,
    #[serde(deserialize_with = "deserialize_path")]
    pub paths: Vec<Vec<(Decimal, Decimal)>>,
} // TODO: T

/// Types of routes
#[derive(Debug, Copy, Clone, Display, From, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i8)]
pub enum RouteType {
    /// Metropolitan train service
    Train = 0,
    /// Metropolitan tram service
    Tram = 1,
    /// Bus Service
    Bus = 2,
    /// V/Line regional train service
    VLine = 3,
    /// Night Bus service
    NightBus = 4,
    /// Other Route Type
    Other(i8),
}

impl Serialize for RouteType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        i8::from(*self).serialize(serializer)
    }
}

//imp deserialize
impl<'de> Deserialize<'de> for RouteType {
    fn deserialize<D>(deserializer: D) -> Result<RouteType, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match i8::deserialize(deserializer)? {
            0 => RouteType::Train,
            1 => RouteType::Tram,
            2 => RouteType::Bus,
            3 => RouteType::VLine,
            4 => RouteType::NightBus,
            x => RouteType::Other(x),
        })
    }
}

impl From<RouteType> for i8 {
    fn from(value: RouteType) -> Self {
        match value {
            RouteType::Train => 0,
            RouteType::Tram => 1,
            RouteType::Bus => 2,
            RouteType::VLine => 3,
            RouteType::NightBus => 4,
            RouteType::Other(x) => x,
        }
    }
}
/// Modes of disruption
#[derive(Debug, Serialize, Deserialize, Clone, From, Copy)]
#[serde(tag = "disruption_mode_name", content = "disruption_mode")]
#[repr(i8)]
pub enum DisruptionModes {
    #[serde(rename = "metro_train")]
    MetroTrain = 1, //    {
    #[serde(rename = "metro_bus")]
    MetroBus = 2,
    #[serde(rename = "metro_tram")]
    MetroTram = 3,
    #[serde(rename = "regional_coach")]
    RegionalCoach = 4,
    #[serde(rename = "regional_train")]
    RegionalTrain = 5,
    #[serde(rename = "regional_bus")]
    RegionalBus = 7,
    #[serde(rename = "school_bus")]
    SchoolBus = 8,
    #[serde(rename = "telebus")]
    Telebus = 9,
    #[serde(rename = "night_bus")]
    NightBus = 10,
    #[serde(rename = "ferry")]
    Ferry = 11,
    #[serde(rename = "interstate_train")]
    InterstateTrain = 12,
    #[serde(rename = "skybus")]
    Skybus = 13,
    #[serde(rename = "taxi")]
    Taxi = 14,
    #[serde(rename = "general")]
    General = 100,
}

impl DisruptionModes {
    pub fn as_number(&self) -> i8 {
        *self as i8
    }
}

//

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Status {
    /// API Version number
    pub version: String,
    /// API system health status (0=offline, 1=online)
    pub health: i8,
}

//

#[derive(Serialize, Default)]
pub struct DeparturesStopOpts {
    /// Filter by platform number at stop
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_numbers: Option<Vec<i32>>,
    /// Filter by identifier of direction of travel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction_id: Option<DirectionId>,
    /// Indicates that stop_id parameter will accept "GTFS stop_id" data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gtfs: Option<bool>,
    /// Filter by the date and time of the request (default = current date and time)
    #[serde(serialize_with = "ser_iso_8601")]
    #[serde(rename = "date_utc")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<NaiveDateTime>,
    /// Maximum number of results returned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<i32>,
    /// Indicates if cancelled services are included in results.
    /// Metro Trains only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_cancelled: Option<bool>,
    /// Indicates if filtering runs to those that arrive at destination before date_urc
    /// (default = false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub look_backwards: Option<bool>,
    /// Last of objects to be returned in full
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<ExpandOptions>>,
    ///Indicates if the route geopath should be returned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_geopath: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiError {
    pub message: String,
    pub status: Status,
}

#[derive(Serialize, Default)]
pub struct DeparturesStopRouteOpts {
    /// Filter by identifier of direction of travel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction_id: Option<DirectionId>,
    /// Indicates that stop_id parameter will accept "GTFS stop_id" data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gtfs: Option<bool>,
    /// Filter by the date and time of the request (default = current date and time)
    #[serde(serialize_with = "ser_iso_8601")]
    #[serde(rename = "date_utc")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<NaiveDateTime>,
    /// Maximum number of results returned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<i32>,
    /// Indicates if cancelled services are included in results.
    /// Metro Trains only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_cancelled: Option<bool>,
    /// Indicates if filtering runs to those that arrive at destination before date_urc
    /// (default = false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub look_backwards: Option<bool>,
    /// Last of objects to be returned in full
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<ExpandOptions>>,
    ///Indicates if the route geopaath should be returned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_geopath: Option<bool>,
}

#[derive(ToAndFro, Serialize)]
pub enum ExpandOptions {
    All,
    Stop,
    Route,
    Run,
    Direction,
    Disruption,
    VehiclePosition,
    VehicleDescriptor,
    None,
}

impl<'de> Deserialize<'de> for ExpandOptions {
    fn deserialize<D>(deserializer: D) -> Result<ExpandOptions, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(serde::de::Error::custom)
    }
}
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeparturesResponse {
    /// Timetabled and real-time service departures
    pub departures: Vec<Departure>,
    /// A train station, tram stop, bus stop, regional coach stop or Night Bus stop
    pub stops: BTreeMap<String, Stop>,
    /// Train lines, tram routes, bus routes, regional coach routes, Night Bus routes
    pub routes: BTreeMap<String, RouteWithGeoPath>,
    /// Individual trips/services of a route
    pub runs: BTreeMap<String, Run>,
    /// Directions of travel of route
    pub directions: BTreeMap<String, Direction>,
    /// Disruption information applicable to relevant routes or stops
    pub disruptions: BTreeMap<String, Disruption>,
    // API Status / Metadata
    pub status: Status,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StoppingPatternsStop {
    #[serde(flatten)]
    pub stop: Stop,
    pub stop_ticket: StopTicket,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stop {
    #[serde(rename = "stop_distance")]
    pub distance: Decimal,
    #[serde(rename = "stop_suburb")]
    pub suburb: String,
    #[serde(rename = "stop_name")]
    pub name: String,
    #[serde(rename = "stop_id")]
    pub id: StopId,
    pub route_type: RouteType,
    #[serde(rename = "stop_latitude")]
    pub latitude: Decimal,
    #[serde(rename = "stop_longitude")]
    pub longitude: Decimal,
    #[serde(rename = "stop_landmark")]
    pub landmark: String,
    #[serde(rename = "stop_sequence")]
    pub sequence: i32,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Departure {
    /// Stop identifier
    pub stop_id: StopId,
    /// Route identifier
    pub route_id: RouteId,
    /// Numeric trip/service run identifier. Defaults to -1 when run identifier is Alphanumeric
    pub run_id: RunId,
    /// Alphanumeric trip/service run identifier
    pub run_ref: String,
    /// Direction of travel identifier
    pub direction_id: DirectionId,
    /// Disruption information identifier(s)
    pub disruption_ids: Vec<DisruptionId>,
    /// Scheduled (i.e. timetabled) departure time and date
    #[serde(deserialize_with = "opt_de_rfc3339")]
    #[serde(serialize_with = "opt_ser_rfc3339")]
    #[serde(rename = "scheduled_departure_utc")]
    pub scheduled_departure: Option<NaiveDateTime>, // TODO: Seems to always be Some
    /// Real-time estimate of departure time and date
    #[serde(deserialize_with = "opt_de_rfc3339")]
    #[serde(serialize_with = "opt_ser_rfc3339")]
    #[serde(rename = "estimated_departure_utc")]
    pub estimated_departure: Option<NaiveDateTime>,
    /// Indicates if the metropolitan train service is at the platform at the time of query.
    /// false for other modes
    pub at_platform: bool,
    /// Platform number at stop (metropolitan train only.
    /// None for other modes
    pub platform_number: Option<String>,
    /// Flag indicating special condition for run
    pub flags: String,
    /// Chronological sequence for the departures in a run.
    pub departure_sequence: i32,

    pub skipped_stops: Option<Vec<Stop>>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StopTicket {
    pub ticket_type: String,
    pub zone: String,
    pub is_free_fare_zone: bool,
    pub ticket_machine: bool,
    pub ticket_checks: bool,
    pub vline_reservation: bool,
    pub ticket_zones: Vec<i32>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Route {
    pub route_type: RouteType,
    #[serde(rename = "route_id")]
    pub id: RouteId,
    #[serde(rename = "route_name")]
    pub name: String,
    #[serde(rename = "route_number")]
    pub number: String,
    #[serde(rename = "route_gtfs_id")]
    pub gtfs_id: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteWithGeoPath {
    #[serde(flatten)]
    pub route: Route,
    /// TODO: T
    pub geopath: Option<Vec<Geopath>>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Direction {
    #[serde(rename = "direction_id")]
    pub id: DirectionId,
    #[serde(rename = "direction_name")]
    pub name: String,
    pub route_id: RouteId,
    pub route_type: RouteType,
}

//

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DirectionsResponse {
    /// Directions of travel of route
    pub directions: Vec<DirectionWithDescription>,
    /// API Status / Metadata
    pub status: Status,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DirectionWithDescription {
    // Direction of travel identifier
    #[serde(flatten)]
    pub direction: Direction,
    /// Description
    #[serde(rename = "route_direction_description")]
    pub description: String,
}

//

#[derive(Serialize, Default)]
pub struct DisruptionsOpts {
    /// Filter by route type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_types: Option<Vec<RouteType>>,
    /// Filter by disruption_modes
    #[serde(rename = "disruption_modes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "ser_disruption_query")]
    pub modes: Option<Vec<DisruptionModes>>,
    /// Filter by status of disruption
    #[serde(rename = "disruption_status")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<DisruptionStatus>,
}

#[derive(Serialize, Default)]
pub struct DisruptionsSpecificOpts {
    /// Filter by status of disruption
    #[serde(rename = "disruption_status")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<DisruptionStatus>,
}

#[derive(ToAndFro, PartialOrd, Ord)]
#[input_case("lower")]
#[output_case("lower")]
pub enum DisruptionStatus {
    Current,
    Planned,
}

impl Serialize for DisruptionStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for DisruptionStatus {
    fn deserialize<D>(deserializer: D) -> Result<DisruptionStatus, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?.to_lowercase();
        Self::from_str(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisruptionsResponse {
    /// Disruption information applicable to relavenet route, run, stop, direction
    pub disruptions: Disruptions,
    /// API Status / Metadata
    pub status: Status,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Disruptions {
    /// Subset of disruption information applicable to multiple route_types
    pub general: Vec<Disruption>,
    /// Subset of disruption information applicable to metropolitan train
    pub metro_train: Vec<Disruption>,
    /// Subset of disruption information applicable to metropolitan tram
    pub metro_tram: Vec<Disruption>,
    /// Subset of disruption information applicable to metropolitan bus
    pub metro_bus: Vec<Disruption>,
    /// Subset of disruption information applicable to V/Line train
    pub regional_train: Vec<Disruption>,
    /// Subset of disruption information applicable to V/Line coach
    pub regional_coach: Vec<Disruption>,
    /// Subset of disruption information applicable to regional bus
    pub regional_bus: Vec<Disruption>,
    /// Subset of disruption information applicable to school bus
    pub school_bus: Vec<Disruption>,
    /// Subset of disruption information applicable to telebus services
    pub telebus: Vec<Disruption>,
    /// Subset of disruption information applicable to night bus
    pub night_bus: Vec<Disruption>,
    /// Subset of disruption information applicable to ferry
    pub ferry: Vec<Disruption>,
    /// Subset of disruption information applicable to interstate train
    pub interstate_train: Vec<Disruption>,
    /// Subset of disruption information applicable to skybus
    pub skybus: Vec<Disruption>,
    /// Subset of disruption information applicable to taxi
    pub taxi: Vec<Disruption>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Disruption {
    /// Disruption information identifier
    pub disruption_id: DisruptionId,
    /// Headline title summarising disruption information
    pub title: String,
    /// URL of relevant article on PTV website
    pub url: String,
    /// Description of the disruption
    pub description: String,
    /// Status of the disruption (e.g. "Planned", "Current")
    pub disruption_status: DisruptionStatus, // TODO: This might want to be a String
    /// Type of disruption
    pub disruption_type: String,
    /// Date and time disruption information is published on PTV website
    #[serde(deserialize_with = "de_rfc3339")]
    #[serde(serialize_with = "ser_rfc3339")]
    pub published_on: NaiveDateTime,
    /// Date and time disruption information was last updated by PTV
    #[serde(deserialize_with = "de_rfc3339")]
    #[serde(serialize_with = "ser_rfc3339")]
    pub last_updated: NaiveDateTime,
    /// Date and time at which disruption begins
    #[serde(deserialize_with = "de_rfc3339")]
    #[serde(serialize_with = "ser_rfc3339")]
    pub from_date: NaiveDateTime,
    /// Date and time at which disruption ends (returns None if unknown)
    #[serde(deserialize_with = "opt_de_rfc3339")]
    #[serde(serialize_with = "opt_ser_rfc3339")]
    pub to_date: Option<NaiveDateTime>,
    /// Route relevant to a disruption (if applicable)
    pub routes: Vec<DisruptionRoute>,
    /// Stop relevant to a disruption (if applicable)
    pub stops: Vec<DisruptionStop>,
    pub colour: String,
    pub display_on_board: bool,
    pub display_status: bool,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisruptionStop {
    #[serde(rename = "stop_id")]
    pub id: StopId,
    #[serde(rename = "stop_name")]
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisruptionRoute {
    #[serde(flatten)]
    pub route: Route,
    /// Direction of travel relevant to disruption
    pub direction: Option<DisruptionDirection>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisruptionDirection {
    /// Route and direction of travel combination identifier
    #[serde(rename = "route_direction_id")]
    pub combination_id: i32,
    /// Direction of travel identifier
    #[serde(rename = "direction_id")]
    pub id: DirectionId,
    /// Name of direction of travel
    #[serde(rename = "direction_name")]
    pub name: String,
    /// Time of service to which disruption applies. Returns None if disruption applies to multiple, or no services
    ///
    /// This doesn't use null, it uses a blank string. I hate it here.
    #[serde(deserialize_with = "de_service_time")]
    pub service_time: Option<NaiveDateTime>,
}

//

#[derive(Serialize, Default)]
pub struct FareEstimateOpts {
    /// Journey touch on
    #[serde(serialize_with = "ser_touch_utc")]
    #[serde(rename = "journey_touch_on_utc")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touch_on: Option<NaiveDateTime>,
    /// Journey touch off
    #[serde(serialize_with = "ser_touch_utc")]
    #[serde(rename = "journey_touch_off_utc")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touch_off: Option<NaiveDateTime>,
    #[serde(rename = "is_journey_in_free_tram_zone")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_tram_zone: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traveled_route_types: Option<Vec<RouteType>>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
pub struct FareEstimateResponse {
    pub fare_estimate_result: FareEstimate,
    // API Status / Metadata
    pub fare_estimate_status: Status,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
pub struct ZoneInfo {
    pub min_zone: i32,
    pub max_zone: i32,
    pub unique_zones: Vec<i32>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum PassengerType {
    Senior,
    Concession,
    FullFare,
}
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
pub struct PassengerFare {
    pub passenger_type: PassengerType,
    pub fare2_hour_off_peak: Decimal,
    pub fare2_hour_peak: Decimal,
    pub fare_daily_peak: Decimal,
    pub fare_daily_off_peak: Decimal,
    pub pass7_days: Decimal,
    pub pass28_to69_day_per_day: Decimal,
    pub pass70_plus_day_per_day: Decimal,
    pub weekend_cap: Decimal,
    pub holiday_cap: Decimal,
}

// TODO: This is undefined on the API documentation
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
pub struct FareEstimate {
    pub is_early_bird: bool,
    pub is_journey_in_free_tram_zone: Option<bool>,
    pub is_this_weekend_journey: Option<bool>,
    pub zone_info: ZoneInfo,
    pub passenger_fares: Vec<PassengerFare>,
}

#[derive(Serialize, Default)]
pub struct OutletsOpts {
    /// Maximum number of results returned
    /// (default = 30)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<i32>,
}

#[derive(Serialize, Default)]
pub struct OutletsLatLongOpts {
    /// Maximum number of results returned
    /// (default = 30)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<i32>,

    /// Maximum distance (in metres) from the specified location
    /// (default = 300)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_distance: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutletsResponse {
    /// Myki ticket outlets
    pub outlets: Vec<Outlet>,
    /// API Status / Metadata
    pub status: Status,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Outlet {
    /// The SLID / SPID
    #[serde(rename = "outlet_slid_spid")]
    pub id: String,
    /// The location name of the outlet
    #[serde(rename = "outlet_name")]
    pub name: String,
    /// The buisness name of the outlet
    #[serde(rename = "outlet_business")]
    pub business: String,
    /// Geographic coordinate of the latitude at outlet
    #[serde(rename = "outlet_latitude")]
    pub latitude: Decimal,
    /// Geographic coordinate of the longitude at outlet
    #[serde(rename = "outlet_longitude")]
    pub longitude: Decimal,
    /// The city/municipality of the outlet
    #[serde(rename = "outlet_suburb")]
    pub suburb: String,
    /// The postcode of the outlet
    #[serde(rename = "outlet_postcode")]
    pub postcode: usize,
    /// The business hours on Monday
    #[serde(rename = "outlet_business_hour_mon")]
    pub hours_monday: Option<String>,
    /// The business hours on Tuesday
    #[serde(rename = "outlet_business_hour_tue")]
    pub hours_tuesday: Option<String>,
    /// The business hours on Wednesday
    #[serde(rename = "outlet_business_hour_wed")]
    pub hours_wednesday: Option<String>,
    /// The business hours on Thursday
    #[serde(rename = "outlet_business_hour_thu")]
    pub hours_thursday: Option<String>,
    /// The business hours on Friday
    #[serde(rename = "outlet_business_hour_fri")]
    pub hours_friday: Option<String>,
    /// The business hours on Saturday
    #[serde(rename = "outlet_business_hour_sat")]
    pub hours_saturday: Option<String>,
    /// The business hours on Sunday
    #[serde(rename = "outlet_business_hour_sun")]
    pub hours_sunday: Option<String>,
    /// Any additional notes for the outlet such as
    /// 'Buy pre-loaded myki cards only'
    #[serde(rename = "outlet_notes")]
    pub note: Option<String>,
}

//

#[derive(Serialize, Default)]
pub struct PatternsRunRouteOpts {
    /// List of objects to be returned in full
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<ExpandOptions>>,
    /// Filter by stop_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<StopId>,
    /// Filter by the date and time of the request
    #[serde(serialize_with = "ser_iso_8601")]
    #[serde(rename = "date_utc")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<NaiveDateTime>,
    /// Include any skipped stops in a stopping pattern
    /// (default = false)
    #[serde(rename = "include_skipped_stops")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_skipped: Option<bool>,
    /// Incidates if the route geopath should be returned
    /// (default = false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_geopath: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)] // PartialOrd, Ord can be added once Value has a strong type
pub struct PatternResponse {
    /// Disruption information applicable to relevant routes or stops
    pub disruptions: Vec<Disruption>,
    /// Timetabled and real-time service departures
    pub departures: Vec<Departure>,
    /// A train station, tram stop, bus stop, regional coach stop or Night Bus stop
    pub stops: BTreeMap<String, StoppingPatternsStop>,
    /// Train lines, tram routes, bus routes, regional coach routes, Night Bus routes
    pub routes: BTreeMap<String, Value>, // TODO needs to be more specific
    /// Individual trips/services of a route
    pub runs: BTreeMap<String, Run>,
    /// Directions of travel of route
    pub directions: BTreeMap<String, Direction>,
    /// API Status / Metadata
    pub status: Status,
}

#[derive(Serialize, Default)]
pub struct RouteOpts {
    /// Filterered by route_types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_types: Option<Vec<RouteType>>,
    /// Filter by name of route.
    /// Accepts partial route name matches
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_name: Option<String>,
}

#[derive(Serialize, Default)]
pub struct RouteIdOpts {
    /// Indicates kif geopath will be returned (default = false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_geopath: Option<bool>,
    // Filter geopath by date (default = current date)
    #[serde(serialize_with = "ser_iso_8601")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<NaiveDateTime>,
}

/// This is just documented wrong?
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoutesResponse {
    /// Train lines, tram routes, bus routes, regional coach routes, Night Bus routes
    pub routes: Vec<RouteWithStatus>,
    /// Documented as route: RouteWithStatus?
    /// API Status / Metadata
    pub status: Status,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoutesIdResponse {
    /// Train lines, tram routes, bus routes, regional coach routes, Night Bus routes
    pub route: Option<RouteWithStatus>,
    /// API Status / Metadata
    pub status: Status,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteWithStatus {
    /// Service status for the route (indicates disruptions)
    #[serde(rename = "route_service_status")]
    pub service_status: RouteServiceStatus,
    #[serde(flatten)]
    pub route: RouteWithGeoPath,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteServiceStatus {
    pub description: String,
    pub timestamp: String, // TODO: Add a deser. No information in docs.
}

#[derive(Serialize, Default)]
pub struct RunsIdOpts {
    /// List of objects to be returned in full
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<ExpandOptions>>,
    /// Filter by the date and time of the request
    #[serde(serialize_with = "ser_iso_8601")]
    #[serde(rename = "date_utc")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<NaiveDateTime>,
}

#[derive(Serialize, Default)]
pub struct RunsRefOpts {
    /// List of objects to be returned in full
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<Vec<ExpandOptions>>,
    /// Filter by the date and time of the request
    #[serde(serialize_with = "ser_iso_8601")]
    #[serde(rename = "date_utc")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<NaiveDateTime>,
    /// Indicates if the route geopath should be returned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_geopath: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RunsResponse {
    /// Individual trips/services of a route
    pub runs: Vec<Run>,
    /// API Status / Metadata
    pub status: Status,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Run {
    /// Numeric trip/service run identifier.
    /// Defaults to -1 when run identifier is Alphanumeric
    pub run_id: RunId,
    /// Alphanumeric trip/service run identifier
    pub run_ref: String,
    /// Route identifier
    pub route_id: RouteId,
    /// Transport mode identifier
    pub route_type: RouteType,
    /// stop_id of final stop of run
    pub final_stop_id: StopId,
    /// Name of destination of run
    pub destination_name: String,
    /// Status of metropolitan train run; returns "scheduled" for other modes
    pub status: String,
    /// Direction of travel identifier
    pub direction_id: DirectionId,
    /// Chronological sequence of the trip/service run on the route in direction
    /// Order ascendingly by this field to get chronological order (earliest first) of runs with the same route_id and direction_id
    ///
    /// What a mouthful
    pub run_sequence: i32,
    // The number of remaining skipped/express stations for the run/service from a stop
    pub express_stop_count: i32,
    // Position of the trip/service run. Available for some Bus, Nightrider and Train runs.
    pub vehicle_position: Option<VehiclePosition>,
    // Descriptor of the trip/service run. Only available for some runs.
    pub vehicle_descriptor: Option<VehicleDescriptor>,
    /// Geopath of the route
    pub geopath: Vec<Geopath>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VehiclePosition {
    /// Geographic coordinate of latitude of the vehicle when known.
    pub latitude: Option<Decimal>,
    /// Geographic coordinate of longitude of the vehicle when known.
    pub longitude: Option<Decimal>,
    /// CIS - Metro Train Vehicle Location Easting coordinate
    pub easting: Option<Decimal>,
    /// CIS - Metro Train Vehicle Location Northing coordinate
    pub northing: Option<Decimal>,
    /// CIS - Metro Train Vehicle Location Direction
    pub direction: Option<String>,
    /// Compass bearing of the vehicle when known, clockwise from True North.
    /// ie. 0 is North and 90 is East
    pub bearing: Option<Decimal>,
    /// Supplier of the vehicle position data
    pub supplier: String,
    /// Date and time that the vehicle position data was supplied
    #[serde(deserialize_with = "de_iso_8601")]
    #[serde(rename = "datetime_utc")]
    pub datetime: NaiveDateTime,
    /// CIS - Metro Train Vehicle Location data expiry time
    pub expiry_time: Option<String>, // TODO: Add a deser. No information in docs.
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceOperator {
    #[serde(rename = "Metro Trains Melbourne")]
    MetroTrainsMelbourne,
    #[serde(rename = "Yarra Trams")]
    YarraTrams,
    #[serde(rename = "Ventura Bus Line")]
    VenturaBusLine,
    Other(String),
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VehicleDescriptor {
    /// Operator name of the vehicle such as "Metro Trains Melbourne", "Yarra Trams", "Ventura Bus Line", etc.
    /// Only available for some runs.
    pub operator: Option<ServiceOperator>,
    /// Operator identifier of the vehicle. Only available for some runs.
    pub id: Option<String>,
    /// Indicator if the vehicle has a low floor. Only available for some tram runs.
    pub low_floor: Option<bool>,
    /// Indicator if the vehicle is air conditioned. Only available for some tram runs.
    pub air_conditioned: Option<bool>,
    /// Vehicle description such as "6 Car Comeng". Only available for some train runs.
    pub description: Option<String>,
    /// Supplier of the vehicle descriptor data
    pub supplier: String,
    /// The length of the vehicle. Applies to CIS - Metro Trains
    /// Meters? Feet? Who knows.
    pub length: Option<String>,
}

#[derive(Serialize, Default)]
pub struct SearchOpts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_types: Option<Vec<RouteType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_distance: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_addresses: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_outlets: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_stop_by_suburb: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_stop_by_gtfs_stop_id: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResultStop {
    #[serde(flatten)]
    pub stop: Stop,
    pub routes: Vec<RouteWithStatus>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SearchResponse {
    pub stops: Vec<ResultStop>,
    pub routes: Vec<RouteWithStatus>,
    pub outlets: Vec<Outlet>,
    pub status: Status,
}

#[derive(Serialize, Default)]
pub struct StopsIdRouteTypeOpts {
    /// Indicates if stop locaiton information should be returned
    #[serde(rename = "stop_location")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<bool>,
    /// Indicates if stop amenities information should be returned
    #[serde(rename = "stop_amenities")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amenities: Option<bool>,
    #[serde(rename = "stop_accessibility")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<bool>,
    #[serde(rename = "stop_contact")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<bool>,
    #[serde(rename = "stop_ticket")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket: Option<bool>,
    #[serde(rename = "stop_staffing")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staffing: Option<bool>,
    #[serde(rename = "stop_disruptions")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disruptions: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StopGps {
    pub latitude: Decimal,
    pub longitude: Decimal,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StopLocation {
    pub gps: StopGps,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StopAmenityDetails {
    pub toilet: bool,
    pub taxi_rank: bool,
    pub car_parking: bool,
    pub cctv: bool,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StopAccessibilityWheelchair {
    pub accessible_ramp: bool,
    pub parking: bool,
    pub telephone: bool,
    pub toilet: bool,
    pub low_ticket_counter: bool,
    pub manouvering: bool,
    pub raised_platform: bool,
    pub ramp: bool,
    pub secondary_path: bool,
    pub raised_platform_shelter: bool,
    pub steep_ramp: bool,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StopAccessibility {
    pub lighting: bool,
    pub platform_number: bool,
    pub escalator: bool,
    pub lift: bool,
    pub stairs: bool,
    pub stop_accessibility: bool,
    pub tactile_ground_surface_indicator: bool,
    pub waiting_room: bool,
    pub wheelchair: StopAccessibilityWheelchair,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StopStaffing {
    pub fri_am_from: String,
    pub fri_am_to: String,
    pub fri_pm_from: String,
    pub fri_pm_to: String,
    pub mon_am_from: String,
    pub mon_am_to: String,
    pub mon_pm_from: String,
    pub mon_pm_to: String,
    pub ph_additional_text: String,
    pub ph_from: String,
    pub ph_to: String,
    pub sat_am_from: String,
    pub sat_am_to: String,
    pub sat_pm_from: String,
    pub sat_pm_to: String,
    pub sun_am_from: String,
    pub sun_am_to: String,
    pub sun_pm_from: String,
    pub sun_pm_to: String,
    pub thu_am_from: String,
    pub thu_am_to: String,
    pub thu_pm_from: String,
    pub thu_pm_to: String,
    pub tue_am_from: String,
    pub tue_am_to: String,
    pub tue_pm_from: String,
    pub tue_pm_to: String,
    pub wed_am_from: String,
    pub wed_am_to: String,
    pub wed_pm_from: String,
    pub wed_pm_to: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StopDetails {
    pub disruption_ids: Vec<DisruptionId>,
    #[serde(rename = "stop_id")]
    pub id: StopId,
    pub station_type: String,
    pub station_description: String,
    pub route_type: RouteType,
    // assumption, because it just says object
    pub routes: Vec<Route>,
    #[serde(rename = "stop_landmark")]
    pub landmark: String,
    #[serde(rename = "stop_name")]
    pub name: String,
    #[serde(rename = "stop_amenities")]
    pub amenities: Option<StopAmenityDetails>,
    #[serde(rename = "stop_accessibility")]
    pub accessibility: Option<StopAccessibility>,
    #[serde(rename = "stop_staffing")]
    pub staffing: Option<StopStaffing>,
    #[serde(rename = "stop_location")]
    pub location: Option<StopLocation>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StopResponse {
    pub stop: StopDetails,
    pub disruptions: Vec<Disruption>,
    pub status: Status,
}
