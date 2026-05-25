use chrono::{Datelike, Days, NaiveDate};

#[macro_export]
macro_rules! reasons {
    ($($x:expr),* $(,)?) => {
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
        Self::parse_const(s).map_err(|e| e.to_string())
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
            idx += 1;
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

#[derive(Debug)]
pub enum TinyVec<T, const N: usize> {
    Inline {
        data: [std::mem::MaybeUninit<T>; N],
        len: usize,
    },
    Heap(Vec<T>),
}

impl<T, const N: usize> TinyVec<T, N> {
    pub fn new() -> Self {
        let data = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        TinyVec::Inline { data, len: 0 }
    }

    pub fn push(&mut self, val: T) {
        match self {
            TinyVec::Inline { data, len } => {
                if *len < N {
                    data[*len] = std::mem::MaybeUninit::new(val);
                    *len += 1;
                } else {
                    let mut vec = Vec::with_capacity(N + 1);
                    for i in 0..N {
                        unsafe {
                            let item = std::ptr::read(data[i].as_ptr());
                            vec.push(item);
                        }
                    }
                    vec.push(val);
                    // Crucial: Set len to 0 before reassigning *self so that when the
                    // old Inline variant is dropped, it doesn't double-free the moved elements.
                    *len = 0;
                    *self = TinyVec::Heap(vec);
                }
            }
            TinyVec::Heap(vec) => {
                vec.push(val);
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            TinyVec::Inline { len, .. } => *len,
            TinyVec::Heap(vec) => vec.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_slice(&self) -> &[T] {
        match self {
            TinyVec::Inline { data, len } => {
                unsafe {
                    let slice = &data[..*len];
                    &*(slice as *const [std::mem::MaybeUninit<T>] as *const [T])
                }
            }
            TinyVec::Heap(vec) => vec.as_slice(),
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match self {
            TinyVec::Inline { data, len } => {
                unsafe {
                    let slice = &mut data[..*len];
                    &mut *(slice as *mut [std::mem::MaybeUninit<T>] as *mut [T])
                }
            }
            TinyVec::Heap(vec) => vec.as_mut_slice(),
        }
    }

    pub fn swap_remove(&mut self, index: usize) -> T {
        match self {
            TinyVec::Inline { data, len } => {
                assert!(index < *len, "swap_remove index out of bounds");
                let last_idx = *len - 1;
                unsafe {
                    let val = std::ptr::read(data[index].as_ptr());
                    if index < last_idx {
                        let last_val = std::ptr::read(data[last_idx].as_ptr());
                        data[index] = std::mem::MaybeUninit::new(last_val);
                    }
                    *len -= 1;
                    val
                }
            }
            TinyVec::Heap(vec) => vec.swap_remove(index),
        }
    }

    pub fn clear(&mut self) {
        match self {
            TinyVec::Inline { data, len } => {
                for i in 0..*len {
                    unsafe {
                        data[i].assume_init_drop();
                    }
                }
                *len = 0;
            }
            TinyVec::Heap(vec) => {
                vec.clear();
            }
        }
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        match self {
            TinyVec::Inline { data, len } => {
                let mut write_idx = 0;
                for read_idx in 0..*len {
                    unsafe {
                        let is_keep = f(&*data[read_idx].as_ptr());
                        if is_keep {
                            if write_idx != read_idx {
                                let val = std::ptr::read(data[read_idx].as_ptr());
                                data[write_idx] = std::mem::MaybeUninit::new(val);
                            }
                            write_idx += 1;
                        } else {
                            data[read_idx].assume_init_drop();
                        }
                    }
                }
                *len = write_idx;
            }
            TinyVec::Heap(vec) => {
                vec.retain(f);
            }
        }
    }
}

impl<T, const N: usize> Default for TinyVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, const N: usize> Clone for TinyVec<T, N> {
    fn clone(&self) -> Self {
        match self {
            TinyVec::Inline { data, len } => {
                let mut new_data: [std::mem::MaybeUninit<T>; N] = unsafe {
                    std::mem::MaybeUninit::uninit().assume_init()
                };
                for i in 0..*len {
                    unsafe {
                        let val = (&*data[i].as_ptr()).clone();
                        new_data[i] = std::mem::MaybeUninit::new(val);
                    }
                }
                TinyVec::Inline { data: new_data, len: *len }
            }
            TinyVec::Heap(vec) => TinyVec::Heap(vec.clone()),
        }
    }
}

impl<T, const N: usize> Drop for TinyVec<T, N> {
    fn drop(&mut self) {
        if let TinyVec::Inline { data, len } = self {
            for i in 0..*len {
                unsafe {
                    data[i].assume_init_drop();
                }
            }
        }
    }
}

impl<T, const N: usize> std::ops::Deref for TinyVec<T, N> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> std::ops::DerefMut for TinyVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> std::iter::FromIterator<T> for TinyVec<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut tiny = TinyVec::new();
        for item in iter {
            tiny.push(item);
        }
        tiny
    }
}

impl<T, const N: usize> From<Vec<T>> for TinyVec<T, N> {
    fn from(vec: Vec<T>) -> Self {
        let mut tiny = TinyVec::new();
        for item in vec {
            tiny.push(item);
        }
        tiny
    }
}

impl<T: serde::Serialize, const N: usize> serde::Serialize for TinyVec<T, N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_slice().serialize(serializer)
    }
}

impl<'de, T: serde::Deserialize<'de> + Default, const N: usize> serde::Deserialize<'de> for TinyVec<T, N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec = Vec::<T>::deserialize(deserializer)?;
        let mut tiny = TinyVec::new();
        for val in vec {
            tiny.push(val);
        }
        Ok(tiny)
    }
}

impl<T, const N: usize> Extend<T> for TinyVec<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

