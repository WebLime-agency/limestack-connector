use serde::{Deserialize, Serialize};

/// Messages from the browser to the connector
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        version: String,
        origin: String,
    },
    GetPrinters,
    Print {
        #[serde(rename = "requestId")]
        request_id: String,
        printer: String,
        format: String,
        data: String, // Base64 encoded
        options: PrintOptions,
    },
    ReadScale,
}

#[derive(Debug, Deserialize)]
pub struct PrintOptions {
    pub copies: Option<u32>,
    #[serde(rename = "paperSize")]
    pub paper_size: Option<String>,
}

/// Messages from the connector to the browser
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        #[serde(rename = "connectorVersion")]
        connector_version: String,
        capabilities: Vec<String>,
        printers: Vec<PrinterInfo>,
    },
    Printers {
        printers: Vec<PrinterInfo>,
    },
    PrintResult {
        #[serde(rename = "requestId")]
        request_id: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    ScaleReading {
        weight: f64,
        unit: String,
        stable: bool,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Serialize, Clone)]
pub struct PrinterInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub printer_type: String,
    pub status: String,
    #[serde(rename = "isDefault")]
    pub is_default: bool,
}
