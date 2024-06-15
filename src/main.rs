use crate::cpu_test::CpuTestResult;

mod cpu_test;
mod mprime;
mod process;
mod ycruncher;

fn main() {
    // TODO:
    // - Prüfe  ob die laufzeit pro core stimmt (vermutung die laufzeit wird nicht richtug aufgeteilt zwischen den methoden)
    // - Wenn die erste test methode failt wird trotzdem die alle anderen test methoden ausgeführt
    let config = cpu_test::CpuTestConfig {
        duration_per_core: "10m".to_string(),
        cores_to_test: vec![],
        test_methods: vec![
            cpu_test::CpuTestMethod::Prime95,
            cpu_test::CpuTestMethod::YCruncher,
        ],
    };

    let rest_result = cpu_test::run(config);

    let mut values: Vec<&CpuTestResult> = rest_result.values().collect();
    values.sort_by(|a, b| a.id.cmp(&b.id));
    for cpu_result in values {
        println!("{:?}", cpu_result);
    }
}
