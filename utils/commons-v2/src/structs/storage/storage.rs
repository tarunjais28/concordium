use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, hash::Hash)]
pub struct Bytes(pub Vec<u8>);

impl schema::SchemaType for Bytes {
    fn get_type() -> schema::Type {
        schema::Type::List(schema::SizeLength::U16, Box::new(schema::Type::U8))
    }
}

impl<const N: usize> From<[u8; N]> for Bytes {
    fn from(bytes: [u8; N]) -> Self {
        Bytes(bytes.into())
    }
}

impl core::ops::Deref for Bytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serial for Bytes {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        let slice: &ByteSlice = self.as_ref();
        slice.serial(out)
    }
}

impl Deserial for Bytes {
    fn deserial<R: Read>(source: &mut R) -> ParseResult<Self> {
        let len = source.read_u16()? as usize;
        let mut buffer = Vec::with_capacity(len);
        // SAFETY:
        // * `param_buffer` has capacity `params_len`
        // * `param_buffer` is fully initialized or error is returned
        unsafe {
            buffer.set_len(len);
            source.read_exact(&mut buffer)?
        }
        Ok(Bytes(buffer))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ByteSlice(pub [u8]);

impl Serial for &ByteSlice {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        out.write_u16(self.0.len() as u16)?;
        out.write_all(&self.0)
    }
}

impl schema::SchemaType for &ByteSlice {
    fn get_type() -> schema::Type {
        schema::Type::List(schema::SizeLength::U16, Box::new(schema::Type::U8))
    }
}

impl<'a> From<&'a [u8]> for &'a ByteSlice {
    fn from(slice: &[u8]) -> Self {
        // SAFETY: ByteSlice just wraps [u8],
        // and &*self.0 is &[u8], therefore
        // transmuting &[u8] to &Slice is safe.
        unsafe { mem::transmute(&*slice) }
    }
}

impl<'a> From<&'a str> for &'a ByteSlice {
    fn from(slice: &'a str) -> Self {
        Self::from(slice.as_bytes())
    }
}

impl PartialEq<Bytes> for ByteSlice {
    fn eq(&self, other: &Bytes) -> bool {
        &self.0 == other.as_slice()
    }
}

impl AsRef<ByteSlice> for [u8] {
    fn as_ref(&self) -> &ByteSlice {
        // SAFETY: ByteSlice just wraps [u8],
        // and &*self.0 is &[u8], therefore
        // transmuting &[u8] to &Slice is safe.
        unsafe { mem::transmute(&*self) }
    }
}

impl AsRef<ByteSlice> for Bytes {
    fn as_ref(&self) -> &ByteSlice {
        self.0.as_slice().as_ref()
    }
}

impl AsRef<ByteSlice> for ByteSlice {
    fn as_ref(&self) -> &ByteSlice {
        self
    }
}

impl AsRef<ByteSlice> for str {
    fn as_ref(&self) -> &ByteSlice {
        self.as_bytes().as_ref()
    }
}

impl<const N: usize> AsRef<ByteSlice> for &[u8; N] {
    fn as_ref(&self) -> &ByteSlice {
        self.as_slice().as_ref()
    }
}

#[derive(Serialize, SchemaType, Debug, Clone, PartialEq, Eq)]
pub struct StorageEntry {
    pub key: Bytes,
    pub value: Bytes,
}

impl StorageEntry {
    pub fn new<K: Serial, V: Serial>(key: &K, value: &V) -> Self {
        Self {
            key: Bytes(to_bytes(key)),
            value: Bytes(to_bytes(value)),
        }
    }
}

#[derive(Debug, Serialize, SchemaType, PartialEq, Eq)]
pub struct StorageEntries {
    pub prefix: Bytes,
    #[concordium(size_length = 2)]
    pub entries: Vec<StorageEntry>,
}

#[derive(Debug, Serial, PartialEq, Eq)]
pub struct StorageEntriesRef<'p, 'l, 'k, 'v> {
    pub prefix: &'p ByteSlice,
    #[concordium(size_length = 2)]
    pub entries: &'l [StorageEntryRef<'k, 'v>],
}

#[derive(Debug, Clone, Serialize, SchemaType, PartialEq, Eq)]
pub enum StorageKeySelection {
    All,
    Some(#[concordium(size_length = 2)] Vec<Bytes>),
}

#[derive(Debug, Clone, Serialize, SchemaType, PartialEq, Eq)]
pub struct StorageKeys {
    pub prefix: Bytes,
    pub keys: StorageKeySelection,
}

impl StorageKeys {
    pub fn some(prefix: Bytes, keys: Vec<Bytes>) -> Self {
        Self {
            prefix,
            keys: StorageKeySelection::Some(keys),
        }
    }

    pub fn all(prefix: Bytes) -> Self {
        Self {
            prefix,
            keys: StorageKeySelection::All,
        }
    }
}

#[derive(Debug, Serial, PartialEq, Eq)]
pub struct StorageKeysRef<'p, 'l, 'k> {
    pub prefix: &'p ByteSlice,
    pub keys: StorageKeySelectionRef<'l, 'k>,
}

impl<'p, 'l, 'k> StorageKeysRef<'p, 'l, 'k> {
    pub fn some(prefix: &'p ByteSlice, keys: &'l [&'k ByteSlice]) -> Self {
        Self {
            prefix,
            keys: StorageKeySelectionRef::Some(keys),
        }
    }

    pub fn all(prefix: &'p ByteSlice) -> Self {
        Self {
            prefix,
            keys: StorageKeySelectionRef::<'static, 'static>::All,
        }
    }
}

#[derive(Debug, Clone, Serial, PartialEq, Eq)]
pub enum StorageKeySelectionRef<'l, 'k> {
    All,
    Some(#[concordium(size_length = 2)] &'l [&'k ByteSlice]),
}

#[derive(Serial, SchemaType, Debug, PartialEq, Eq)]
pub struct StorageEntryRef<'k, 'v> {
    pub key: &'k ByteSlice,
    pub value: &'v ByteSlice,
}

impl<'k, 'v> StorageEntryRef<'k, 'v> {
    pub fn new(key: &'k ByteSlice, value: &'v ByteSlice) -> Self {
        StorageEntryRef { key, value }
    }
}

#[derive(Debug, Clone, Serialize, SchemaType, PartialEq, Eq)]
pub struct StorageGetEntryResult {
    pub prefix: Bytes,
    #[concordium(size_length = 2)]
    pub entries: Vec<MaybeStorageEntry>,
}

#[derive(Debug, Clone, Serialize, SchemaType, PartialEq, Eq)]
pub struct MaybeStorageEntry {
    pub key: Bytes,
    pub value: Option<Bytes>,
}
