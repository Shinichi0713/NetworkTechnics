use std::error::Error;

pub fn load_csv(path: &str) -> Result<Vec<f64>, Box<dyn Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)?;
    let mut values = Vec::new();
    for result in rdr.records() {
        let record = result?;
        if let Some(field) = record.get(0) {
            if let Ok(val) = field.trim().parse::<f64>() {
                values.push(val);
            }
        }
    }
    Ok(values)
}

pub fn save_csv(path: &str, values: &[f64]) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path(path)?;
    for v in values {
        wtr.write_record(&[v.to_string()])?;
    }
    wtr.flush()?;
    Ok(())
}
