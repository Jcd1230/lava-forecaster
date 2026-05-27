use crate::date_utils::TimePeriod;

#[derive(Debug, Clone, Copy)]
pub struct CompiledDoseRule {
    pub dose_number: usize,
    pub absolute_minimum_age: Option<TimePeriod>,
    pub minimum_age: Option<TimePeriod>,
    pub earliest_recommended_age: Option<TimePeriod>,
    pub latest_recommended_age: Option<TimePeriod>,
    pub allowed_cvx: &'static [u16],
}

#[derive(Debug, Clone, Copy)]
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
    pub name: &'static str,
    pub code: &'static str,
    pub vaccine_group: &'static str,
    pub num_doses: usize,
    pub doses: Vec<CompiledDoseRule>,
    pub intervals: Vec<CompiledDoseInterval>,
    pub max_age_clamp: Option<(TimePeriod, crate::models::SeriesStatus)>,
}

// Fluent builders to make writing schedules in Rust extremely clean
impl CompiledSeries {
    pub fn builder(name: &'static str) -> CompiledSeriesBuilder {
        CompiledSeriesBuilder::new(name)
    }
}

pub struct CompiledSeriesBuilder {
    name: &'static str,
    code: Option<&'static str>,
    vaccine_group: Option<&'static str>,
    num_doses: usize,
    doses: Vec<CompiledDoseRule>,
    intervals: Vec<CompiledDoseInterval>,
    max_age_clamp: Option<(TimePeriod, crate::models::SeriesStatus)>,
}

impl CompiledSeriesBuilder {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            code: None,
            vaccine_group: None,
            num_doses: 0,
            doses: Vec::new(),
            intervals: Vec::new(),
            max_age_clamp: None,
        }
    }

    pub fn code(mut self, code: &'static str) -> Self {
        self.code = Some(code);
        self
    }

    pub fn vaccine_group(mut self, group: &'static str) -> Self {
        self.vaccine_group = Some(group);
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

    pub fn max_age_clamp(mut self, age: TimePeriod, status: crate::models::SeriesStatus) -> Self {
        self.max_age_clamp = Some((age, status));
        self
    }

    pub fn build(self) -> CompiledSeries {
        let code = self.code.unwrap_or(self.name);
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
            max_age_clamp: self.max_age_clamp,
        }
    }
}

pub struct CompiledDoseRuleBuilder {
    dose_number: usize,
    absolute_minimum_age: Option<TimePeriod>,
    minimum_age: Option<TimePeriod>,
    earliest_recommended_age: Option<TimePeriod>,
    latest_recommended_age: Option<TimePeriod>,
    allowed_cvx: &'static [u16],
}

impl CompiledDoseRuleBuilder {
    pub fn new(dose_number: usize) -> Self {
        Self {
            dose_number,
            absolute_minimum_age: None,
            minimum_age: None,
            earliest_recommended_age: None,
            latest_recommended_age: None,
            allowed_cvx: &[],
        }
    }

    pub fn abs_min_age(mut self, age: TimePeriod) -> Self {
        self.absolute_minimum_age = Some(age);
        self
    }

    pub fn min_age(mut self, age: TimePeriod) -> Self {
        self.minimum_age = Some(age);
        self
    }

    pub fn earliest_recommended_age(mut self, age: TimePeriod) -> Self {
        self.earliest_recommended_age = Some(age);
        self
    }

    pub fn latest_recommended_age(mut self, age: TimePeriod) -> Self {
        self.latest_recommended_age = Some(age);
        self
    }

    pub fn cvx(mut self, codes: &'static [u16]) -> Self {
        self.allowed_cvx = codes;
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

    pub fn abs_min_interval(mut self, int: TimePeriod) -> Self {
        self.absolute_minimum_interval = Some(int);
        self
    }

    pub fn min_interval(mut self, int: TimePeriod) -> Self {
        self.minimum_interval = Some(int);
        self
    }

    pub fn earliest_recommended_interval(mut self, int: TimePeriod) -> Self {
        self.earliest_recommended_interval = Some(int);
        self
    }

    pub fn latest_recommended_interval(mut self, int: TimePeriod) -> Self {
        self.latest_recommended_interval = Some(int);
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
