use serde::{Serialize, Serializer};
use std::io::BufRead;

#[derive(Debug, PartialEq, PartialOrd)]
pub enum Coverage {
    Covered,
    NotCovered,
    NoExecutableCode,
}

#[derive(Debug)]
pub struct LineInfo {
    pub line_number: u32,
    pub statement: String,
    pub coverage: Coverage,
}
#[derive(Debug, Serialize)]
pub struct StatementInfo {
    pub line_number: u32,
    statement: String,
    failed_tests: u32,
    passed_tests: u32,
    total_failed: u32,
    #[serde(serialize_with = "round_serialize")]
    pub suspiciousness: f32,
}

fn round_serialize<S>(x: &f32, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Round to 2 decimal places
    s.serialize_str(&format!("{:.2}", x))
}

impl StatementInfo {
    pub fn new(line_number: u32, statement: String, total_failed: u32) -> StatementInfo {
        let passed_tests = 0;
        let failed_tests = 0;
        let suspiciousness = 0.0;
        StatementInfo {
            line_number,
            statement,
            failed_tests,
            passed_tests,
            total_failed,
            suspiciousness,
        }
    }
    pub fn add_passing_coverage(&mut self) {
        self.passed_tests += 1;
    }
    pub fn add_failing_coverage(&mut self) {
        self.failed_tests += 1;
    }
    pub fn calculate_suspiciousness(&mut self) {
        let failed_tests = self.failed_tests as f32;
        let passed_tests = self.passed_tests as f32;
        let total_failed = self.total_failed as f32;
        let suspiciousness =
            (failed_tests * failed_tests) / (passed_tests + total_failed - failed_tests);
        self.suspiciousness = suspiciousness;
    }
}

fn parse_gcov_line(line: &str) -> LineInfo {
    let line = line.split(':').collect::<Vec<&str>>();
    let coverage_str = line[0].trim();
    // The value can be one of three things:
    // It could be a number, indicating the number of times the line was executed
    // It could be #####, indicating the line was never executed
    // It could be a dash, indicating the line has no executable code
    let coverage;
    match coverage_str {
        "-" => coverage = Coverage::NoExecutableCode,
        "#####" => coverage = Coverage::NotCovered,
        _ => coverage = Coverage::Covered,
    }
    let line_number = line[1].trim().parse::<u32>().unwrap();
    // combine the rest of the line into a single string
    let mut statement = String::new();
    for i in 2..line.len() {
        match i {
            2 => statement.push_str(line[i].trim_start()),
            _ => statement.push_str(&format!(":{}", line[i])),
        }
    }
    let line_info = LineInfo {
        line_number,
        statement,
        coverage,
    };
    line_info
}

pub fn parse_gcov_file(path: &std::path::PathBuf) -> Vec<LineInfo> {
    let mut lines = Vec::new();
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let line = line.unwrap();
        if line.is_empty() {
            continue;
        }
        let line_info = parse_gcov_line(&line);
        // if this is a line with no executable code, skip it
        if line_info.coverage == Coverage::NoExecutableCode {
            continue;
        }
        // if this is a line with line number 0, skip it
        if line_info.line_number == 0 {
            continue;
        }
        lines.push(line_info);
    }
    lines
}

pub fn add_test_to_statements(
    statements: &mut Vec<StatementInfo>,
    tests: &Vec<LineInfo>,
    is_passing: bool,
) {
    // the two vectors should be the same length
    assert_eq!(statements.len(), tests.len());
    for i in 0..statements.len() {
        if tests[i].coverage == Coverage::Covered {
            if is_passing {
                statements[i].add_passing_coverage();
            } else {
                statements[i].add_failing_coverage();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::INFINITY;

    use super::*;
    #[test]
    fn test_parse_gcov_line_no_executable() {
        let line = "        -:    0:Source:tcas4.c";
        let line_info = parse_gcov_line(line);
        assert_eq!(line_info.line_number, 0);
        assert_eq!(line_info.statement, "Source:tcas4.c");
        assert_eq!(line_info.coverage, Coverage::NoExecutableCode);
    }
    #[test]
    fn test_parse_gcov_line_not_covered() {
        let line = "    #####:   77:	result = Own_Above_Threat() && (Cur_Vertical_Sep >= MINSEP) && (Up_Separation >= ALIM());";
        let line_info = parse_gcov_line(line);
        assert_eq!(line_info.line_number, 77);
        assert_eq!(line_info.statement, "result = Own_Above_Threat() && (Cur_Vertical_Sep >= MINSEP) && (Up_Separation >= ALIM());");
        assert_eq!(line_info.coverage, Coverage::NotCovered);
    }

    #[test]
    fn test_parse_gcov_line_covered() {
        let line = "        2:   61:    return (Climb_Inhibit ? Up_Separation + NOZCROSS : Up_Separation);";
        let line_info = parse_gcov_line(line);
        assert_eq!(line_info.line_number, 61);
        assert_eq!(
            line_info.statement,
            "return (Climb_Inhibit ? Up_Separation + NOZCROSS : Up_Separation);"
        );
        assert_eq!(line_info.coverage, Coverage::Covered);
    }

    // Tests for DStar calculation
    #[test]
    fn test_dstar_calculation() {
        let mut statement_info = StatementInfo::new(1, "test".to_string(), 2);
        statement_info.add_passing_coverage();
        statement_info.add_passing_coverage();
        statement_info.add_passing_coverage();
        statement_info.add_failing_coverage();
        // Result should be (1 * 1) / (3 + 2 - 1) = 0.25
        statement_info.calculate_suspiciousness();
        assert_eq!(statement_info.suspiciousness, 0.25);
    }
    #[test]
    fn test_dstar_calculation_from_hw() {
        let mut statement_info = StatementInfo::new(1, "test".to_string(), 617);
        for i in 0..616 {
            statement_info.add_failing_coverage()
        }
        // Result should be (616) / (0 + 617 - 616) = 0.25
        statement_info.calculate_suspiciousness();
        assert_eq!(statement_info.suspiciousness, 379456.00);
    }

    #[test]
    fn test_dstar_calculation_zero() {
        let mut statement_info = StatementInfo::new(1, "test".to_string(), 2);
        statement_info.add_passing_coverage();
        statement_info.add_passing_coverage();
        statement_info.add_passing_coverage();
        // Result should be (0 * 0) / (3 + 2 - 1) = 0
        statement_info.calculate_suspiciousness();
        assert_eq!(statement_info.suspiciousness, 0.00);
    }

    #[test]
    fn test_dstar_calculation_zero_divide() {
        let mut statement_info = StatementInfo::new(1, "test".to_string(), 3);
        statement_info.add_failing_coverage();
        statement_info.add_failing_coverage();
        statement_info.add_failing_coverage();
        // Result should be (3 * 3) / (3 + 0 - 3) = infinity
        statement_info.calculate_suspiciousness();
        assert_eq!(statement_info.suspiciousness, INFINITY);
    }

    #[test]
    fn test_add_test_to_statement() {
        let mut statements = Vec::new();
        statements.push(StatementInfo::new(1, "test".to_string(), 2));
        statements.push(StatementInfo::new(2, "test".to_string(), 2));
        statements.push(StatementInfo::new(3, "test".to_string(), 2));
        let mut tests = Vec::new();
        tests.push(LineInfo {
            line_number: 1,
            statement: "test".to_string(),
            coverage: Coverage::Covered,
        });
        tests.push(LineInfo {
            line_number: 2,
            statement: "test".to_string(),
            coverage: Coverage::NotCovered,
        });
        tests.push(LineInfo {
            line_number: 3,
            statement: "test".to_string(),
            coverage: Coverage::Covered,
        });
        add_test_to_statements(&mut statements, &tests, true);
        assert_eq!(statements[0].passed_tests, 1);
        assert_eq!(statements[0].failed_tests, 0);
        assert_eq!(statements[1].passed_tests, 0);
        assert_eq!(statements[1].failed_tests, 0);
        assert_eq!(statements[2].passed_tests, 1);
        assert_eq!(statements[2].failed_tests, 0);
    }
}
