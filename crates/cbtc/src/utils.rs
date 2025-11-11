/// Extract amount from a contract's interface views
pub fn extract_amount(contract: &ledger::models::JsActiveContract) -> Option<f64> {
    if let Some(views) = &contract.created_event.interface_views {
        for view in views {
            if let Some(Some(value)) = &view.view_value {
                if let Some(amount_value) = value.get("amount") {
                    if let Some(amount_str) = amount_value.as_str() {
                        return amount_str.parse::<f64>().ok();
                    } else if let Some(amount_f64) = amount_value.as_f64() {
                        return Some(amount_f64);
                    }
                }
            }
        }
    }
    None
}
