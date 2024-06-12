use crate::cpu_test::CpuTestResult;

mod cpu_test;
mod mprime;

fn main() {
    let rest_result = cpu_test::run("10m", vec![]);

    let mut values: Vec<&CpuTestResult> = rest_result.values().collect();
    values.sort_by(|a, b| a.id.cmp(&b.id));
    for cpu_result in values {
        println!("{:?}", cpu_result);
    }
}
