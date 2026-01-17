use anyhow::Result;
use chrono::{Datelike, NaiveDate};

/// Trait for periodic note types
pub trait PeriodType: Sized {
    /// Get the identifier for this period (e.g., "2026-W03", "2026-01-16", "2026-Q1")
    fn identifier(&self) -> String;

    /// Get the display string for list output (includes date range for weekly/quarterly)
    fn display_string(&self) -> String;

    /// Parse a date string and return the period it belongs to
    fn from_date_str(date_str: &str) -> Result<Self>;

    /// Get the current period
    fn current() -> Self;

    /// Get the previous period
    fn prev(&self) -> Self;

    /// Get the next period
    fn next(&self) -> Self;

    /// Get the filename for this period's note
    fn filename(&self) -> String {
        format!("{}.md", self.identifier())
    }

    /// Get the template name for this period type
    fn template_name() -> &'static str;
}

/// Daily note period
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Daily {
    date: NaiveDate,
}

impl Daily {
    pub fn from_date(date: NaiveDate) -> Self {
        Self { date }
    }
}

impl PeriodType for Daily {
    fn identifier(&self) -> String {
        self.date.format("%Y-%m-%d").to_string()
    }

    fn display_string(&self) -> String {
        self.identifier()
    }

    fn from_date_str(date_str: &str) -> Result<Self> {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
        Ok(Self::from_date(date))
    }

    fn current() -> Self {
        Self::from_date(chrono::Local::now().date_naive())
    }

    fn prev(&self) -> Self {
        Self::from_date(self.date - chrono::Duration::days(1))
    }

    fn next(&self) -> Self {
        Self::from_date(self.date + chrono::Duration::days(1))
    }

    fn template_name() -> &'static str {
        "daily"
    }
}

/// Weekly note period (ISO week)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weekly {
    year: i32,
    week: u32,
}

impl Weekly {
    pub fn from_date(date: NaiveDate) -> Self {
        let iso_week = date.iso_week();
        Self {
            year: iso_week.year(),
            week: iso_week.week(),
        }
    }

    fn monday(&self) -> NaiveDate {
        NaiveDate::from_isoywd_opt(self.year, self.week, chrono::Weekday::Mon).unwrap()
    }

    fn sunday(&self) -> NaiveDate {
        NaiveDate::from_isoywd_opt(self.year, self.week, chrono::Weekday::Sun).unwrap()
    }
}

impl PeriodType for Weekly {
    fn identifier(&self) -> String {
        format!("{}-W{:02}", self.year, self.week)
    }

    fn display_string(&self) -> String {
        let monday = self.monday();
        let sunday = self.sunday();
        format!(
            "{}    {} - {}",
            self.identifier(),
            monday.format("%b %d"),
            sunday.format("%b %d")
        )
    }

    fn from_date_str(date_str: &str) -> Result<Self> {
        // Try to parse as week identifier first (e.g., "2026-W03")
        if date_str.contains("-W") {
            let parts: Vec<&str> = date_str.split("-W").collect();
            if parts.len() == 2
                && let (Ok(year), Ok(week)) = (parts[0].parse::<i32>(), parts[1].parse::<u32>())
            {
                return Ok(Self { year, week });
            }
        }

        // Fall back to parsing as date string
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
        Ok(Self::from_date(date))
    }

    fn current() -> Self {
        Self::from_date(chrono::Local::now().date_naive())
    }

    fn prev(&self) -> Self {
        let monday = self.monday();
        Self::from_date(monday - chrono::Duration::days(7))
    }

    fn next(&self) -> Self {
        let monday = self.monday();
        Self::from_date(monday + chrono::Duration::days(7))
    }

    fn template_name() -> &'static str {
        "weekly"
    }
}

/// Quarterly note period
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quarterly {
    year: i32,
    quarter: u32, // 1-4
}

impl Quarterly {
    pub fn from_date(date: NaiveDate) -> Self {
        let quarter = ((date.month() - 1) / 3) + 1;
        Self {
            year: date.year(),
            quarter,
        }
    }
}

impl PeriodType for Quarterly {
    fn identifier(&self) -> String {
        format!("{}-Q{}", self.year, self.quarter)
    }

    fn display_string(&self) -> String {
        let months = match self.quarter {
            1 => "Jan - Mar",
            2 => "Apr - Jun",
            3 => "Jul - Sep",
            4 => "Oct - Dec",
            _ => unreachable!(),
        };
        format!("{}     {}", self.identifier(), months)
    }

    fn from_date_str(date_str: &str) -> Result<Self> {
        let date_str = date_str.trim();

        // Try to parse as quarter identifier (e.g., "2026-Q1")
        if date_str.contains("-Q") {
            let parts: Vec<&str> = date_str.split("-Q").collect();
            if parts.len() == 2
                && let (Ok(year), Ok(quarter)) = (parts[0].parse::<i32>(), parts[1].parse::<u32>())
                && (1..=4).contains(&quarter)
            {
                return Ok(Self { year, quarter });
            }
        }

        // Handle quarter shortcuts (q1, Q1, etc.)
        let date_str_lower = date_str.to_lowercase();
        if date_str_lower.starts_with('q') {
            let quarter_str = date_str_lower.strip_prefix('q').unwrap();
            let quarter: u32 = quarter_str.parse()?;
            if !(1..=4).contains(&quarter) {
                anyhow::bail!("Quarter must be between 1 and 4");
            }
            let year = chrono::Local::now().year();
            return Ok(Self { year, quarter });
        }

        // Fall back to parsing as date string
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
        Ok(Self::from_date(date))
    }

    fn current() -> Self {
        Self::from_date(chrono::Local::now().date_naive())
    }

    fn prev(&self) -> Self {
        if self.quarter == 1 {
            Self {
                year: self.year - 1,
                quarter: 4,
            }
        } else {
            Self {
                year: self.year,
                quarter: self.quarter - 1,
            }
        }
    }

    fn next(&self) -> Self {
        if self.quarter == 4 {
            Self {
                year: self.year + 1,
                quarter: 1,
            }
        } else {
            Self {
                year: self.year,
                quarter: self.quarter + 1,
            }
        }
    }

    fn template_name() -> &'static str {
        "quarterly"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_identifier() {
        let daily = Daily::from_date(NaiveDate::from_ymd_opt(2026, 1, 16).unwrap());
        assert_eq!(daily.identifier(), "2026-01-16");
        assert_eq!(daily.filename(), "2026-01-16.md");
    }

    #[test]
    fn test_weekly_identifier() {
        let weekly = Weekly::from_date(NaiveDate::from_ymd_opt(2026, 1, 16).unwrap());
        assert_eq!(weekly.identifier(), "2026-W03");
        assert_eq!(weekly.filename(), "2026-W03.md");
    }

    #[test]
    fn test_quarterly_identifier() {
        let quarterly = Quarterly::from_date(NaiveDate::from_ymd_opt(2026, 1, 16).unwrap());
        assert_eq!(quarterly.identifier(), "2026-Q1");
        assert_eq!(quarterly.filename(), "2026-Q1.md");
    }

    #[test]
    fn test_daily_navigation() {
        let daily = Daily::from_date(NaiveDate::from_ymd_opt(2026, 1, 16).unwrap());
        let prev = daily.prev();
        let next = daily.next();

        assert_eq!(prev.identifier(), "2026-01-15");
        assert_eq!(next.identifier(), "2026-01-17");
    }

    #[test]
    fn test_weekly_navigation() {
        let weekly = Weekly::from_date(NaiveDate::from_ymd_opt(2026, 1, 16).unwrap());
        let prev = weekly.prev();
        let next = weekly.next();

        assert_eq!(prev.identifier(), "2026-W02");
        assert_eq!(next.identifier(), "2026-W04");
    }

    #[test]
    fn test_quarterly_navigation() {
        let q1 = Quarterly::from_date(NaiveDate::from_ymd_opt(2026, 1, 16).unwrap());
        let prev = q1.prev();
        let next = q1.next();

        assert_eq!(prev.identifier(), "2025-Q4");
        assert_eq!(next.identifier(), "2026-Q2");
    }

    #[test]
    fn test_quarterly_shortcuts() {
        let q1 = Quarterly::from_date_str("q1").unwrap();
        let q4 = Quarterly::from_date_str("Q4").unwrap();

        assert_eq!(q1.quarter, 1);
        assert_eq!(q4.quarter, 4);
    }

    #[test]
    fn test_weekly_display() {
        let weekly = Weekly::from_date(NaiveDate::from_ymd_opt(2026, 1, 16).unwrap());
        let display = weekly.display_string();

        assert!(display.contains("2026-W03"));
        assert!(display.contains("Jan 12"));
        assert!(display.contains("Jan 18"));
    }

    #[test]
    fn test_quarterly_display() {
        let q1 = Quarterly::from_date(NaiveDate::from_ymd_opt(2026, 1, 16).unwrap());
        let display = q1.display_string();

        assert!(display.contains("2026-Q1"));
        assert!(display.contains("Jan - Mar"));
    }
}
