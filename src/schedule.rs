use crate::date_utils::TimePeriod;

#[derive(Debug, Clone)]
pub struct CompiledDoseRule {
    pub dose_number: usize,
    pub absolute_minimum_age: Option<TimePeriod>,
    pub minimum_age: Option<TimePeriod>,
    pub earliest_recommended_age: Option<TimePeriod>,
    pub latest_recommended_age: Option<TimePeriod>,
    pub allowed_cvx: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompiledDoseInterval {
    pub from_dose: usize,
    pub to_dose: usize,
    pub absolute_minimum_interval: Option<TimePeriod>,
    pub minimum_interval: Option<TimePeriod>,
    pub earliest_recommended_interval: Option<TimePeriod>,
    pub latest_recommended_interval: Option<TimePeriod>,
}

#[derive(Debug, Clone)]
pub struct CompiledSeries {
    pub name: String,
    pub code: String,
    pub vaccine_group: String,
    pub num_doses: usize,
    pub doses: Vec<CompiledDoseRule>,
    pub intervals: Vec<CompiledDoseInterval>,
}

// Fluent builders to make writing schedules in Rust extremely clean
impl CompiledSeries {
    pub fn builder(name: &str) -> CompiledSeriesBuilder {
        CompiledSeriesBuilder::new(name)
    }
}

pub struct CompiledSeriesBuilder {
    name: String,
    code: Option<String>,
    vaccine_group: Option<String>,
    num_doses: usize,
    doses: Vec<CompiledDoseRule>,
    intervals: Vec<CompiledDoseInterval>,
}

impl CompiledSeriesBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            code: None,
            vaccine_group: None,
            num_doses: 0,
            doses: Vec::new(),
            intervals: Vec::new(),
        }
    }

    pub fn code(mut self, code: &str) -> Self {
        self.code = Some(code.to_string());
        self
    }

    pub fn vaccine_group(mut self, group: &str) -> Self {
        self.vaccine_group = Some(group.to_string());
        self
    }

    pub fn num_doses(mut self, num: usize) -> Self {
        self.num_doses = num;
        self
    }

    pub fn dose<F>(mut self, num: usize, f: F) -> Self
    where
        F: FnOnce(CompiledDoseRuleBuilder) -> CompiledDoseRuleBuilder,
    {
        let builder = CompiledDoseRuleBuilder::new(num);
        let rule = f(builder).build();
        self.doses.push(rule);
        self
    }

    pub fn interval<F>(mut self, from: usize, to: usize, f: F) -> Self
    where
        F: FnOnce(CompiledDoseIntervalBuilder) -> CompiledDoseIntervalBuilder,
    {
        let builder = CompiledDoseIntervalBuilder::new(from, to);
        let interval = f(builder).build();
        self.intervals.push(interval);
        self
    }

    pub fn build(self) -> CompiledSeries {
        let code = self.code.unwrap_or_else(|| self.name.clone());
        let vaccine_group = self.vaccine_group.expect("vaccine_group is required");
        
        // Sort doses and intervals to ensure they are in correct order
        let mut doses = self.doses;
        doses.sort_by_key(|d| d.dose_number);
        let mut intervals = self.intervals;
        intervals.sort_by_key(|i| (i.from_dose, i.to_dose));

        CompiledSeries {
            name: self.name,
            code,
            vaccine_group,
            num_doses: self.num_doses,
            doses,
            intervals,
        }
    }
}

pub struct CompiledDoseRuleBuilder {
    dose_number: usize,
    absolute_minimum_age: Option<TimePeriod>,
    minimum_age: Option<TimePeriod>,
    earliest_recommended_age: Option<TimePeriod>,
    latest_recommended_age: Option<TimePeriod>,
    allowed_cvx: Vec<String>,
}

impl CompiledDoseRuleBuilder {
    pub fn new(dose_number: usize) -> Self {
        Self {
            dose_number,
            absolute_minimum_age: None,
            minimum_age: None,
            earliest_recommended_age: None,
            latest_recommended_age: None,
            allowed_cvx: Vec::new(),
        }
    }

    pub fn abs_min_age(mut self, age: &str) -> Self {
        self.absolute_minimum_age = Some(TimePeriod::parse(age).unwrap());
        self
    }

    pub fn min_age(mut self, age: &str) -> Self {
        self.minimum_age = Some(TimePeriod::parse(age).unwrap());
        self
    }

    pub fn earliest_recommended_age(mut self, age: &str) -> Self {
        self.earliest_recommended_age = Some(TimePeriod::parse(age).unwrap());
        self
    }

    pub fn latest_recommended_age(mut self, age: &str) -> Self {
        self.latest_recommended_age = Some(TimePeriod::parse(age).unwrap());
        self
    }

    pub fn cvx(mut self, codes: &[&str]) -> Self {
        self.allowed_cvx = codes.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn build(self) -> CompiledDoseRule {
        CompiledDoseRule {
            dose_number: self.dose_number,
            absolute_minimum_age: self.absolute_minimum_age,
            minimum_age: self.minimum_age,
            earliest_recommended_age: self.earliest_recommended_age,
            latest_recommended_age: self.latest_recommended_age,
            allowed_cvx: self.allowed_cvx,
        }
    }
}

pub struct CompiledDoseIntervalBuilder {
    from_dose: usize,
    to_dose: usize,
    absolute_minimum_interval: Option<TimePeriod>,
    minimum_interval: Option<TimePeriod>,
    earliest_recommended_interval: Option<TimePeriod>,
    latest_recommended_interval: Option<TimePeriod>,
}

impl CompiledDoseIntervalBuilder {
    pub fn new(from_dose: usize, to_dose: usize) -> Self {
        Self {
            from_dose,
            to_dose,
            absolute_minimum_interval: None,
            minimum_interval: None,
            earliest_recommended_interval: None,
            latest_recommended_interval: None,
        }
    }

    pub fn abs_min_interval(mut self, int: &str) -> Self {
        self.absolute_minimum_interval = Some(TimePeriod::parse(int).unwrap());
        self
    }

    pub fn min_interval(mut self, int: &str) -> Self {
        self.minimum_interval = Some(TimePeriod::parse(int).unwrap());
        self
    }

    pub fn earliest_recommended_interval(mut self, int: &str) -> Self {
        self.earliest_recommended_interval = Some(TimePeriod::parse(int).unwrap());
        self
    }

    pub fn latest_recommended_interval(mut self, int: &str) -> Self {
        self.latest_recommended_interval = Some(TimePeriod::parse(int).unwrap());
        self
    }

    pub fn build(self) -> CompiledDoseInterval {
        CompiledDoseInterval {
            from_dose: self.from_dose,
            to_dose: self.to_dose,
            absolute_minimum_interval: self.absolute_minimum_interval,
            minimum_interval: self.minimum_interval,
            earliest_recommended_interval: self.earliest_recommended_interval,
            latest_recommended_interval: self.latest_recommended_interval,
        }
    }
}
