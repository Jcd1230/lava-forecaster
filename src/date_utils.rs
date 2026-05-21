use chrono::{Datelike, Days, NaiveDate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurationUnit {
    Days,
    Weeks,
    Months,
    Years,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimePeriodPart {
    pub value: i32,
    pub unit: DurationUnit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimePeriod {
    pub parts: Vec<TimePeriodPart>,
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
        _ => panic!("Invalid month: {}", month),
    }
}

pub fn add_months(date: NaiveDate, duration: i32) -> NaiveDate {
    if duration == 0 {
        return date;
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
    
    let mut result = NaiveDate::from_ymd_opt(year, month as u32, target_day).unwrap();
    if rolled_over {
        // In ICE, adding months that clamp to the end-of-month rolls over by +1 day (e.g. Aug 31 + 1m -> Oct 1)
        result = result.succ_opt().unwrap();
    }
    result
}

pub fn add_years(date: NaiveDate, duration: i32) -> NaiveDate {
    if duration == 0 {
        return date;
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
    
    let mut result = NaiveDate::from_ymd_opt(year, month, target_day).unwrap();
    if rolled_over {
        result = result.succ_opt().unwrap();
    }
    result
}

impl TimePeriod {
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut parts = Vec::new();
        let chars: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
        let mut i = 0;
        
        while i < chars.len() {
            // Read sign if present, or number
            let mut sign = 1;
            if chars[i] == '+' {
                i += 1;
            } else if chars[i] == '-' {
                sign = -1;
                i += 1;
            }
            
            if i >= chars.len() || !chars[i].is_ascii_digit() {
                return Err(format!("Expected digit at position {} in {}", i, s));
            }
            
            let mut num_str = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                num_str.push(chars[i]);
                i += 1;
            }
            
            let num: i32 = num_str.parse().map_err(|e| format!("{}", e))?;
            let signed_value = num * sign;
            
            if i >= chars.len() {
                return Err(format!("Expected unit character at the end of {}", s));
            }
            
            let unit = match chars[i].to_ascii_lowercase() {
                'd' => DurationUnit::Days,
                'w' => DurationUnit::Weeks,
                'm' => DurationUnit::Months,
                'y' => DurationUnit::Years,
                _ => return Err(format!("Unknown unit character '{}'", chars[i])),
            };
            i += 1;
            
            parts.push(TimePeriodPart {
                value: signed_value,
                unit,
            });
        }
        
        Ok(TimePeriod { parts })
    }

    pub fn add_to(&self, mut date: NaiveDate) -> NaiveDate {
        for part in &self.parts {
            match part.unit {
                DurationUnit::Days => {
                    if part.value >= 0 {
                        date = date.checked_add_days(Days::new(part.value as u64)).unwrap();
                    } else {
                        date = date.checked_sub_days(Days::new((-part.value) as u64)).unwrap();
                    }
                }
                DurationUnit::Weeks => {
                    let total_days = part.value * 7;
                    if total_days >= 0 {
                        date = date.checked_add_days(Days::new(total_days as u64)).unwrap();
                    } else {
                        date = date.checked_sub_days(Days::new((-total_days) as u64)).unwrap();
                    }
                }
                DurationUnit::Months => {
                    date = add_months(date, part.value);
                }
                DurationUnit::Years => {
                    date = add_years(date, part.value);
                }
            }
        }
        date
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
        let tp1 = TimePeriod::parse("42d").unwrap();
        assert_eq!(tp1.add_to(birth), NaiveDate::from_ymd_opt(2020, 2, 12).unwrap());
        
        // Sign prefix
        let tp2 = TimePeriod::parse("2m").unwrap();
        assert_eq!(tp2.add_to(birth), NaiveDate::from_ymd_opt(2020, 3, 1).unwrap());
        
        // Plus/Minus
        let tp3 = TimePeriod::parse("4y-4d").unwrap();
        assert_eq!(tp3.add_to(birth), NaiveDate::from_ymd_opt(2023, 12, 28).unwrap());
        
        // Rollover month add logic: August 31st + 1 month -> October 1st
        let aug31 = NaiveDate::from_ymd_opt(2020, 8, 31).unwrap();
        let tp_1m = TimePeriod::parse("1m").unwrap();
        assert_eq!(tp_1m.add_to(aug31), NaiveDate::from_ymd_opt(2020, 10, 1).unwrap());
        
        // August 30th + 1 month -> September 30th (no rollover)
        let aug30 = NaiveDate::from_ymd_opt(2020, 8, 30).unwrap();
        assert_eq!(tp_1m.add_to(aug30), NaiveDate::from_ymd_opt(2020, 9, 30).unwrap());
    }
}
