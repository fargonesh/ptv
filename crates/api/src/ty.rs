//! Some types are marked as 'Value' and have a note, TODO: T.
//! This is because the API documentation does not specify the type of the value.
//! I'll do my best to fill in what appears to be correct, but it's not guaranteed to be correct.
//!
//! I appreciate any work done to fill in the TODO: T types.

use chrono::NaiveDate;
use derive_more::{Display, From};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use to_and_fro::ToAndFro;

use crate::helpers::deserialize_path;

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
pub enum DisruptionMode {
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

impl DisruptionMode {
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
#[derive(ToAndFro, PartialOrd, Ord, Serialize, Deserialize)]
#[input_case("lower")]
#[output_case("lower")]
pub enum DisruptionStatus {
    Current,
    Planned,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiError {
    pub message: String,
    pub status: Status,
}

#[derive(ToAndFro, Serialize, Deserialize)]
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum PassengerType {
    Senior,
    Concession,
    FullFare,
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
