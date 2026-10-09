use myprecision_core::sensors::DcimRow;
use serde::Deserialize;

use super::WmiReaders;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NumericSensor {
    element_name: String,
    current_reading: i64,
}

impl WmiReaders {
    /// Raw `DCIM_NumericSensor` rows; `None` when Dell Command | Monitor is unavailable.
    pub fn dcim(&self) -> Option<Vec<DcimRow>> {
        let rows: Vec<NumericSensor> =
            self.dcim.as_ref()?.raw_query("SELECT ElementName, CurrentReading FROM DCIM_NumericSensor").ok()?;
        Some(
            rows.into_iter()
                .map(|r| DcimRow { element_name: r.element_name, current_reading: r.current_reading })
                .collect(),
        )
    }
}
