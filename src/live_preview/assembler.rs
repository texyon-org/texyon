use std::{fmt, sync::Arc};

const PREAMBLE: u8 = 247;
const POSTAMBLE: u8 = 248;
const POST_POSTAMBLE: u8 = 249;
const BEGIN_PAGE: u8 = 139;
const FILL: u8 = 223;
const NO_PAGE: u32 = u32::MAX;
const POSTAMBLE_FIXED_LEN: usize = 29;
const BEGIN_PAGE_FIXED_LEN: usize = 45;

/// The concrete stream format identified by an assembled document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XdvFileType {
    /// XeTeX's extended DVI format.
    Xdv,
    /// Tectonic's semantically paginated XDV format.
    Spx,
}

impl XdvFileType {
    fn from_id(id: u8) -> Result<Self, XdvAssemblyError> {
        match id {
            7 => Ok(Self::Xdv),
            100 => Ok(Self::Spx),
            _ => Err(XdvAssemblyError::UnsupportedFileType(id)),
        }
    }
}

/// Input accepted by [`DocumentAssembler`].
#[derive(Clone, Copy, Debug)]
pub enum DocumentFragment<'a> {
    /// A complete XDV/SPX document.
    Complete(&'a [u8]),
    /// A replacement tail whose offsets are already absolute document offsets.
    ///
    /// The bytes before `prefix_len` are retained from the previous coherent
    /// document. The combined stream is fully revalidated before it can be
    /// observed by a caller.
    Tail {
        /// Number of bytes retained from the previous document.
        prefix_len: usize,
        /// Complete replacement bytes from `prefix_len` through EOF.
        bytes: &'a [u8],
    },
}

/// A coherent, structurally validated XDV/SPX document revision.
#[derive(Clone, Debug)]
pub struct AssembledDocument {
    bytes: Arc<[u8]>,
    file_type: XdvFileType,
    page_offsets: Arc<[u32]>,
    postamble_offset: u32,
    post_postamble_offset: u32,
}

impl AssembledDocument {
    /// Exact immutable document bytes.
    pub fn bytes(&self) -> &Arc<[u8]> {
        &self.bytes
    }

    /// Stream format declared in the preamble and post-postamble.
    pub fn file_type(&self) -> XdvFileType {
        self.file_type
    }

    /// Absolute BOP offsets in document order.
    pub fn page_offsets(&self) -> &[u32] {
        &self.page_offsets
    }

    /// Absolute postamble offset.
    pub fn postamble_offset(&self) -> u32 {
        self.postamble_offset
    }

    /// Absolute post-postamble offset.
    pub fn post_postamble_offset(&self) -> u32 {
        self.post_postamble_offset
    }
}

/// Structural failure while assembling an XDV/SPX revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XdvAssemblyError {
    /// A tail was supplied without a previous coherent document.
    MissingBaseDocument,
    /// The requested retained prefix is longer than the base document.
    PrefixOutOfBounds {
        /// Requested retained prefix length.
        requested: usize,
        /// Available base document length.
        available: usize,
    },
    /// A stream offset cannot be represented by XDV's 32-bit pointers.
    DocumentTooLarge(usize),
    /// A required field extends beyond the supplied bytes.
    Truncated {
        /// Name of the field being read.
        field: &'static str,
        /// Byte offset at which the field should begin.
        offset: usize,
    },
    /// An opcode or identifier was not present where required.
    UnexpectedByte {
        /// Name of the field being checked.
        field: &'static str,
        /// Absolute byte offset of the field.
        offset: usize,
        /// Required byte value.
        expected: u8,
        /// Observed byte value.
        actual: u8,
    },
    /// The stream uses neither the XDV nor SPX identifier.
    UnsupportedFileType(u8),
    /// Preamble and post-postamble identifiers disagree.
    FileTypeMismatch {
        /// Identifier declared by the preamble.
        preamble: u8,
        /// Identifier repeated by the trailer.
        trailer: u8,
    },
    /// Fewer than four DVI fill bytes terminate the stream.
    MissingFillBytes,
    /// An embedded absolute pointer is outside its legal range.
    InvalidPointer {
        /// Name of the pointer being checked.
        field: &'static str,
        /// Invalid absolute offset.
        value: u32,
    },
    /// The postamble page count and BOP chain disagree.
    PageCountMismatch {
        /// Page count declared by the postamble.
        declared: u16,
        /// Page count found by traversing the BOP chain.
        discovered: usize,
    },
}

impl fmt::Display for XdvAssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBaseDocument => write!(f, "an XDV tail requires a base document"),
            Self::PrefixOutOfBounds {
                requested,
                available,
            } => write!(
                f,
                "XDV prefix {requested} exceeds base document length {available}"
            ),
            Self::DocumentTooLarge(size) => {
                write!(f, "XDV document length {size} exceeds 32-bit offsets")
            }
            Self::Truncated { field, offset } => {
                write!(f, "truncated XDV {field} at byte {offset}")
            }
            Self::UnexpectedByte {
                field,
                offset,
                expected,
                actual,
            } => write!(
                f,
                "invalid XDV {field} at byte {offset}: expected {expected}, found {actual}"
            ),
            Self::UnsupportedFileType(id) => write!(f, "unsupported XDV file type {id}"),
            Self::FileTypeMismatch { preamble, trailer } => write!(
                f,
                "XDV preamble type {preamble} disagrees with trailer type {trailer}"
            ),
            Self::MissingFillBytes => write!(f, "XDV stream has fewer than four fill bytes"),
            Self::InvalidPointer { field, value } => {
                write!(f, "invalid XDV {field} pointer {value}")
            }
            Self::PageCountMismatch {
                declared,
                discovered,
            } => write!(
                f,
                "XDV declares {declared} pages but its BOP chain contains {discovered}"
            ),
        }
    }
}

impl std::error::Error for XdvAssemblyError {}

/// Validates and assembles immutable XDV/SPX document revisions.
#[derive(Clone, Copy, Debug, Default)]
pub struct DocumentAssembler;

impl DocumentAssembler {
    /// Construct an assembler.
    pub fn new() -> Self {
        Self
    }

    /// Assemble `fragment` against the last coherent document, if needed.
    pub fn assemble(
        &self,
        previous: Option<&AssembledDocument>,
        fragment: DocumentFragment<'_>,
    ) -> Result<AssembledDocument, XdvAssemblyError> {
        let bytes: Arc<[u8]> = match fragment {
            DocumentFragment::Complete(bytes) => Arc::from(bytes),
            DocumentFragment::Tail { prefix_len, bytes } => {
                let previous = previous.ok_or(XdvAssemblyError::MissingBaseDocument)?;
                if prefix_len > previous.bytes.len() {
                    return Err(XdvAssemblyError::PrefixOutOfBounds {
                        requested: prefix_len,
                        available: previous.bytes.len(),
                    });
                }
                let capacity = prefix_len
                    .checked_add(bytes.len())
                    .ok_or(XdvAssemblyError::DocumentTooLarge(usize::MAX))?;
                if capacity > u32::MAX as usize {
                    return Err(XdvAssemblyError::DocumentTooLarge(capacity));
                }
                let mut combined = Vec::with_capacity(capacity);
                combined.extend_from_slice(&previous.bytes[..prefix_len]);
                combined.extend_from_slice(bytes);
                Arc::from(combined)
            }
        };

        validate(bytes)
    }
}

fn validate(bytes: Arc<[u8]>) -> Result<AssembledDocument, XdvAssemblyError> {
    if bytes.len() > u32::MAX as usize {
        return Err(XdvAssemblyError::DocumentTooLarge(bytes.len()));
    }
    expect_byte(&bytes, 0, PREAMBLE, "preamble opcode")?;
    let preamble_id = byte(&bytes, 1, "preamble identifier")?;
    let file_type = XdvFileType::from_id(preamble_id)?;

    let fill_count = bytes.iter().rev().take_while(|&&byte| byte == FILL).count();
    if fill_count < 4 {
        return Err(XdvAssemblyError::MissingFillBytes);
    }
    let fill_offset = bytes.len() - fill_count;
    let post_postamble = fill_offset
        .checked_sub(6)
        .ok_or(XdvAssemblyError::Truncated {
            field: "post-postamble",
            offset: fill_offset,
        })?;
    expect_byte(
        &bytes,
        post_postamble,
        POST_POSTAMBLE,
        "post-postamble opcode",
    )?;
    let postamble = read_u32(&bytes, post_postamble + 1, "postamble")?;
    let trailer_id = byte(&bytes, post_postamble + 5, "trailer identifier")?;
    if trailer_id != preamble_id {
        return Err(XdvAssemblyError::FileTypeMismatch {
            preamble: preamble_id,
            trailer: trailer_id,
        });
    }
    let postamble_usize = postamble as usize;
    if postamble_usize <= 1
        || postamble_usize
            .checked_add(POSTAMBLE_FIXED_LEN)
            .is_none_or(|end| end > post_postamble)
    {
        return Err(XdvAssemblyError::InvalidPointer {
            field: "postamble",
            value: postamble,
        });
    }
    expect_byte(&bytes, postamble_usize, POSTAMBLE, "postamble opcode")?;

    let last_bop = read_u32(&bytes, postamble_usize + 1, "last BOP")?;
    let declared_pages = read_u16(&bytes, postamble_usize + 27, "page count")?;
    let mut reverse_pages = Vec::with_capacity(declared_pages as usize);
    let mut current = last_bop;

    while current != NO_PAGE {
        let offset = current as usize;
        if offset <= 1
            || offset
                .checked_add(BEGIN_PAGE_FIXED_LEN)
                .is_none_or(|end| end > postamble_usize)
        {
            return Err(XdvAssemblyError::InvalidPointer {
                field: "BOP",
                value: current,
            });
        }
        expect_byte(&bytes, offset, BEGIN_PAGE, "BOP opcode")?;
        reverse_pages.push(current);
        if reverse_pages.len() > declared_pages as usize {
            return Err(XdvAssemblyError::PageCountMismatch {
                declared: declared_pages,
                discovered: reverse_pages.len(),
            });
        }
        let previous = read_u32(&bytes, offset + 41, "previous BOP")?;
        if previous != NO_PAGE && previous >= current {
            return Err(XdvAssemblyError::InvalidPointer {
                field: "previous BOP",
                value: previous,
            });
        }
        current = previous;
    }

    if reverse_pages.len() != declared_pages as usize {
        return Err(XdvAssemblyError::PageCountMismatch {
            declared: declared_pages,
            discovered: reverse_pages.len(),
        });
    }
    reverse_pages.reverse();

    Ok(AssembledDocument {
        bytes,
        file_type,
        page_offsets: Arc::from(reverse_pages),
        postamble_offset: postamble,
        post_postamble_offset: post_postamble as u32,
    })
}

fn byte(bytes: &[u8], offset: usize, field: &'static str) -> Result<u8, XdvAssemblyError> {
    bytes
        .get(offset)
        .copied()
        .ok_or(XdvAssemblyError::Truncated { field, offset })
}

fn expect_byte(
    bytes: &[u8],
    offset: usize,
    expected: u8,
    field: &'static str,
) -> Result<(), XdvAssemblyError> {
    let actual = byte(bytes, offset, field)?;
    if actual != expected {
        return Err(XdvAssemblyError::UnexpectedByte {
            field,
            offset,
            expected,
            actual,
        });
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize, field: &'static str) -> Result<u16, XdvAssemblyError> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or(XdvAssemblyError::Truncated { field, offset })?;
    Ok(u16::from_be_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize, field: &'static str) -> Result<u32, XdvAssemblyError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(XdvAssemblyError::Truncated { field, offset })?;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_XDV: &[u8] = include_bytes!("../../tests/tex-outputs/the_letter_a.xdv");

    #[test]
    fn validates_real_document_and_page_chain() {
        let document = DocumentAssembler::new()
            .assemble(None, DocumentFragment::Complete(REAL_XDV))
            .unwrap();

        assert_eq!(document.file_type(), XdvFileType::Xdv);
        assert_eq!(document.page_offsets().len(), 1);
        assert_eq!(document.bytes().as_ref(), REAL_XDV);
        assert_eq!(
            document.bytes()[document.postamble_offset() as usize],
            POSTAMBLE
        );
    }

    #[test]
    fn reconstructs_and_revalidates_tail() {
        let assembler = DocumentAssembler::new();
        let document = assembler
            .assemble(None, DocumentFragment::Complete(REAL_XDV))
            .unwrap();
        let split = document.page_offsets()[0] as usize;
        let rebuilt = assembler
            .assemble(
                Some(&document),
                DocumentFragment::Tail {
                    prefix_len: split,
                    bytes: &REAL_XDV[split..],
                },
            )
            .unwrap();

        assert_eq!(rebuilt.bytes().as_ref(), REAL_XDV);
        assert_eq!(rebuilt.page_offsets(), document.page_offsets());
    }

    #[test]
    fn rejects_corrupt_trailer_pointer_without_losing_base() {
        let assembler = DocumentAssembler::new();
        let document = assembler
            .assemble(None, DocumentFragment::Complete(REAL_XDV))
            .unwrap();
        let mut corrupt = REAL_XDV.to_vec();
        let pointer = document.post_postamble_offset() as usize + 1;
        corrupt[pointer..pointer + 4].copy_from_slice(&u32::MAX.to_be_bytes());

        let error = assembler
            .assemble(None, DocumentFragment::Complete(&corrupt))
            .unwrap_err();
        assert_eq!(
            error,
            XdvAssemblyError::InvalidPointer {
                field: "postamble",
                value: u32::MAX,
            }
        );
        assert_eq!(document.bytes().as_ref(), REAL_XDV);
    }

    #[test]
    fn rejects_forward_or_cyclic_bop_links() {
        let assembler = DocumentAssembler::new();
        let document = assembler
            .assemble(None, DocumentFragment::Complete(REAL_XDV))
            .unwrap();
        let mut corrupt = REAL_XDV.to_vec();
        let bop = document.page_offsets()[0];
        let pointer = bop as usize + 41;
        corrupt[pointer..pointer + 4].copy_from_slice(&bop.to_be_bytes());

        assert!(matches!(
            assembler.assemble(None, DocumentFragment::Complete(&corrupt)),
            Err(XdvAssemblyError::InvalidPointer {
                field: "previous BOP",
                ..
            })
        ));
    }

    #[test]
    fn tail_requires_a_valid_base_and_prefix() {
        let assembler = DocumentAssembler::new();
        assert_eq!(
            assembler
                .assemble(
                    None,
                    DocumentFragment::Tail {
                        prefix_len: 0,
                        bytes: REAL_XDV,
                    },
                )
                .unwrap_err(),
            XdvAssemblyError::MissingBaseDocument
        );

        let document = assembler
            .assemble(None, DocumentFragment::Complete(REAL_XDV))
            .unwrap();
        assert!(matches!(
            assembler.assemble(
                Some(&document),
                DocumentFragment::Tail {
                    prefix_len: REAL_XDV.len() + 1,
                    bytes: &[],
                },
            ),
            Err(XdvAssemblyError::PrefixOutOfBounds { .. })
        ));
    }
}
