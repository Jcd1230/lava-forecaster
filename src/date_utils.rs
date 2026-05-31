use chrono::{Datelike, Days, NaiveDate};

#[macro_export]
macro_rules! reasons {
    () => {
        $crate::date_utils::TinyVec::new()
    };
    ($($x:expr),+ $(,)?) => {
        {
            let mut r = $crate::date_utils::TinyVec::new();
            $(
                r.push($x.into());
            )*
            r
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    Days,
    Weeks,
    Months,
    Years,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimePeriodPart {
    pub value: i32,
    pub unit: DurationUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimePeriod {
    pub parts: [Option<TimePeriodPart>; 2],
}

#[macro_export]
macro_rules! time_period {
    ($s:expr) => {
        {
            const TP: $crate::date_utils::TimePeriod = match $crate::date_utils::TimePeriod::parse_const($s) {
                Ok(tp) => tp,
                Err(e) => panic!("{}", e),
            };
            TP
        }
    };
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        // Safety: callers always pass month values derived from NaiveDate::month() (1..=12)
        _ => unreachable!("Invalid month: {}", month),
    }
}

pub fn add_months(date: NaiveDate, duration: i32) -> Result<NaiveDate, crate::errors::ForecasterError> {
    if duration == 0 {
        return Ok(date);
    }
    
    let day_before = date.day();
    let mut year = date.year();
    let mut month = date.month() as i32 + duration;
    
    while month > 12 {
        month -= 12;
        year += 1;
    }
    while month < 1 {
        month += 12;
        year -= 1;
    }
    
    let last_day = last_day_of_month(year, month as u32);
    let mut target_day = day_before;
    let mut rolled_over = false;
    
    if target_day > last_day {
        target_day = last_day;
        rolled_over = true;
    }
    
    let mut result = NaiveDate::from_ymd_opt(year, month as u32, target_day)
        .ok_or_else(|| crate::errors::ForecasterError::DateArithmeticError {
            operation: "add_months",
            detail: format!("could not construct date {}-{:02}-{:02}", year, month, target_day),
        })?;
    if rolled_over {
        // In ICE, adding months that clamp to the end-of-month rolls over by +1 day (e.g. Aug 31 + 1m -> Oct 1)
        result = result.succ_opt()
            .ok_or_else(|| crate::errors::ForecasterError::DateArithmeticError {
                operation: "add_months",
                detail: format!("successor of {} overflows", result),
            })?;
    }
    Ok(result)
}

pub fn add_years(date: NaiveDate, duration: i32) -> Result<NaiveDate, crate::errors::ForecasterError> {
    if duration == 0 {
        return Ok(date);
    }
    
    let day_before = date.day();
    let year = date.year() + duration;
    let month = date.month();
    
    let last_day = last_day_of_month(year, month);
    let mut target_day = day_before;
    let mut rolled_over = false;
    
    if target_day > last_day {
        target_day = last_day;
        rolled_over = true;
    }
    
    let mut result = NaiveDate::from_ymd_opt(year, month, target_day)
        .ok_or_else(|| crate::errors::ForecasterError::DateArithmeticError {
            operation: "add_years",
            detail: format!("could not construct date {}-{:02}-{:02}", year, month, target_day),
        })?;
    if rolled_over {
        result = result.succ_opt()
            .ok_or_else(|| crate::errors::ForecasterError::DateArithmeticError {
                operation: "add_years",
                detail: format!("successor of {} overflows", result),
            })?;
    }
    Ok(result)
}

/// Convenience wrapper: adds years to a date, panicking only on truly impossible
/// date combinations (e.g. NaiveDate::MAX + 1y). Safe for clinical age computations
/// where inputs are valid patient birth dates and offsets are small (< 200y).
#[inline]
pub fn add_years_unchecked(date: NaiveDate, duration: i32) -> NaiveDate {
    add_years(date, duration).expect("date arithmetic overflow in add_years_unchecked")
}

/// Convenience wrapper: adds months to a date, panicking only on truly impossible
/// date combinations. Safe for clinical age computations where inputs are valid
/// patient birth dates and offsets are small.
#[inline]
pub fn add_months_unchecked(date: NaiveDate, duration: i32) -> NaiveDate {
    add_months(date, duration).expect("date arithmetic overflow in add_months_unchecked")
}

impl TimePeriod {
    pub fn parse(s: &str) -> Result<Self, crate::errors::ForecasterError> {
        Self::parse_const(s).map_err(|e| crate::errors::ForecasterError::InvalidTimePeriod(e.to_string()))
    }

    pub const fn parse_const(s: &str) -> Result<Self, &'static str> {
        let bytes = s.as_bytes();
        let mut parts = [None, None];
        let mut part_idx = 0;
        let mut i = 0;
        
        while i < bytes.len() {
            // Skip whitespace
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }
            
            let mut sign = 1;
            if bytes[i] == b'+' {
                i += 1;
            } else if bytes[i] == b'-' {
                sign = -1;
                i += 1;
            }
            
            // Skip whitespace after sign
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            
            if i >= bytes.len() || bytes[i] < b'0' || bytes[i] > b'9' {
                return Err("Expected digit");
            }
            
            let mut num: i32 = 0;
            while i < bytes.len() && bytes[i] >= b'0' && bytes[i] <= b'9' {
                num = num * 10 + (bytes[i] - b'0') as i32;
                i += 1;
            }
            
            let signed_value = num * sign;
            
            // Skip whitespace before unit
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            
            if i >= bytes.len() {
                return Err("Expected unit character");
            }
            
            let unit = match bytes[i] {
                b'd' | b'D' => DurationUnit::Days,
                b'w' | b'W' => DurationUnit::Weeks,
                b'm' | b'M' => DurationUnit::Months,
                b'y' | b'Y' => DurationUnit::Years,
                _ => return Err("Unknown unit character"),
            };
            i += 1;
            
            if part_idx >= 2 {
                return Err("TimePeriod cannot have more than 2 parts in this representation");
            }
            parts[part_idx] = Some(TimePeriodPart { value: signed_value, unit });
            part_idx += 1;
        }
        
        Ok(TimePeriod { parts })
    }

    pub fn add_to(&self, mut date: NaiveDate) -> NaiveDate {
        let mut idx = 0;
        while idx < self.parts.len() {
            if let Some(ref part) = self.parts[idx] {
                match part.unit {
                    DurationUnit::Days => {
                        if part.value >= 0 {
                            date = date.checked_add_days(Days::new(part.value as u64))
                                .expect("date overflow in add_to (days+)");
                        } else {
                            date = date.checked_sub_days(Days::new((-part.value) as u64))
                                .expect("date overflow in add_to (days-)");
                        }
                    }
                    DurationUnit::Weeks => {
                        let total_days = part.value * 7;
                        if total_days >= 0 {
                            date = date.checked_add_days(Days::new(total_days as u64))
                                .expect("date overflow in add_to (weeks+)");
                        } else {
                            date = date.checked_sub_days(Days::new((-total_days) as u64))
                                .expect("date overflow in add_to (weeks-)");
                        }
                    }
                    DurationUnit::Months => {
                        date = add_months(date, part.value)
                            .expect("date overflow in add_to (months)");
                    }
                    DurationUnit::Years => {
                        date = add_years(date, part.value)
                            .expect("date overflow in add_to (years)");
                    }
                }
            }
            idx += 1;
        }
        date
    }

    /// Fallible version of `add_to` that returns a `Result` instead of panicking.
    /// Use this in contexts where graceful error propagation is preferred.
    pub fn try_add_to(&self, mut date: NaiveDate) -> Result<NaiveDate, crate::errors::ForecasterError> {
        let mut idx = 0;
        while idx < self.parts.len() {
            if let Some(ref part) = self.parts[idx] {
                match part.unit {
                    DurationUnit::Days => {
                        if part.value >= 0 {
                            date = date.checked_add_days(Days::new(part.value as u64))
                                .ok_or_else(|| crate::errors::ForecasterError::DateArithmeticError {
                                    operation: "add_to",
                                    detail: format!("adding {} days to {} overflows", part.value, date),
                                })?;
                        } else {
                            date = date.checked_sub_days(Days::new((-part.value) as u64))
                                .ok_or_else(|| crate::errors::ForecasterError::DateArithmeticError {
                                    operation: "add_to",
                                    detail: format!("subtracting {} days from {} overflows", -part.value, date),
                                })?;
                        }
                    }
                    DurationUnit::Weeks => {
                        let total_days = part.value * 7;
                        if total_days >= 0 {
                            date = date.checked_add_days(Days::new(total_days as u64))
                                .ok_or_else(|| crate::errors::ForecasterError::DateArithmeticError {
                                    operation: "add_to",
                                    detail: format!("adding {} weeks to {} overflows", part.value, date),
                                })?;
                        } else {
                            date = date.checked_sub_days(Days::new((-total_days) as u64))
                                .ok_or_else(|| crate::errors::ForecasterError::DateArithmeticError {
                                    operation: "add_to",
                                    detail: format!("subtracting {} weeks from {} overflows", -part.value, date),
                                })?;
                        }
                    }
                    DurationUnit::Months => {
                        date = add_months(date, part.value)?;
                    }
                    DurationUnit::Years => {
                        date = add_years(date, part.value)?;
                    }
                }
            }
            idx += 1;
        }
        Ok(date)
    }
}

pub fn compare_elapsed(d1: NaiveDate, d2: NaiveDate, period: &TimePeriod) -> std::cmp::Ordering {
    let target = period.add_to(d1);
    d2.cmp(&target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_add() {
        let birth = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        
        // Single unit
        let tp1 = crate::time_period!("42d");
        assert_eq!(tp1.add_to(birth), NaiveDate::from_ymd_opt(2020, 2, 12).unwrap());
        
        // Sign prefix
        let tp2 = crate::time_period!("2m");
        assert_eq!(tp2.add_to(birth), NaiveDate::from_ymd_opt(2020, 3, 1).unwrap());
        
        // Plus/Minus
        let tp3 = crate::time_period!("4y-4d");
        assert_eq!(tp3.add_to(birth), NaiveDate::from_ymd_opt(2023, 12, 28).unwrap());
        
        // Rollover month add logic: August 31st + 1 month -> October 1st
        let aug31 = NaiveDate::from_ymd_opt(2020, 8, 31).unwrap();
        let tp_1m = crate::time_period!("1m");
        assert_eq!(tp_1m.add_to(aug31), NaiveDate::from_ymd_opt(2020, 10, 1).unwrap());
        
        // August 30th + 1 month -> September 30th (no rollover)
        let aug30 = NaiveDate::from_ymd_opt(2020, 8, 30).unwrap();
        assert_eq!(tp_1m.add_to(aug30), NaiveDate::from_ymd_opt(2020, 9, 30).unwrap());
    }
}

pub type TinyVec<T, const N: usize> = smallvec::SmallVec<[T; N]>;

