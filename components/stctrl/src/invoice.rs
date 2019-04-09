use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use app::ser_string::{invoice_id_to_string, string_to_invoice_id, SerStringError};

use app::invoice::InvoiceId;

use toml;

#[derive(Debug, PartialEq, Eq)]
pub struct Invoice {
    pub invoice_id: InvoiceId,
    pub dest_payment: u128,
}

#[derive(Debug)]
pub enum InvoiceFileError {
    IoError(io::Error),
    TomlDeError(toml::de::Error),
    TomlSeError(toml::ser::Error),
    SerStringError,
    ParseDestPaymentError,
    InvalidPublicKey,
}

/// A helper structure for serialize and deserializing Invoice.
#[derive(Serialize, Deserialize)]
pub struct InvoiceFile {
    pub invoice_id: String,
    pub dest_payment: String,
}

impl From<io::Error> for InvoiceFileError {
    fn from(e: io::Error) -> Self {
        InvoiceFileError::IoError(e)
    }
}

impl From<toml::de::Error> for InvoiceFileError {
    fn from(e: toml::de::Error) -> Self {
        InvoiceFileError::TomlDeError(e)
    }
}

impl From<toml::ser::Error> for InvoiceFileError {
    fn from(e: toml::ser::Error) -> Self {
        InvoiceFileError::TomlSeError(e)
    }
}

impl From<SerStringError> for InvoiceFileError {
    fn from(_e: SerStringError) -> Self {
        InvoiceFileError::SerStringError
    }
}

/// Load Invoice from a file
pub fn load_invoice_from_file(path: &Path) -> Result<Invoice, InvoiceFileError> {
    let data = fs::read_to_string(&path)?;
    let invoice_file: InvoiceFile = toml::from_str(&data)?;

    let invoice_id = string_to_invoice_id(&invoice_file.invoice_id)?;
    let dest_payment = invoice_file
        .dest_payment
        .parse()
        .map_err(|_| InvoiceFileError::ParseDestPaymentError)?;

    Ok(Invoice {
        invoice_id,
        dest_payment,
    })
}

/// Store Invoice to file
pub fn store_invoice_to_file(invoice: &Invoice, path: &Path) -> Result<(), InvoiceFileError> {
    let Invoice {
        ref invoice_id,
        dest_payment,
    } = invoice;

    let invoice_file = InvoiceFile {
        invoice_id: invoice_id_to_string(&invoice_id),
        dest_payment: dest_payment.to_string(),
    };

    let data = toml::to_string(&invoice_file)?;

    let mut file = File::create(path)?;
    file.write(&data.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use app::invoice::{InvoiceId, INVOICE_ID_LEN};

    #[test]
    fn test_invoice_file_basic() {
        let invoice_file: InvoiceFile = toml::from_str(
            r#"
            invoice_id = 'invoice_id'
            dest_payment = '100'
        "#,
        )
        .unwrap();

        assert_eq!(invoice_file.invoice_id, "invoice_id");
        assert_eq!(invoice_file.dest_payment, "100");
    }

    #[test]
    fn test_store_load_invoice() {
        // Create a temporary directory:
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invoice_file");

        let invoice = Invoice {
            invoice_id: InvoiceId::from(&[1; INVOICE_ID_LEN]),
            dest_payment: 100,
        };

        store_invoice_to_file(&invoice, &file_path).unwrap();
        let invoice2 = load_invoice_from_file(&file_path).unwrap();

        assert_eq!(invoice, invoice2);
    }
}
