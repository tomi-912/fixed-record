use crate::error::Error;
use std::fmt;

/// A fixed-width byte buffer whose size `N` is known at compile time.
/// コンパイル時にサイズ `N` が決定される固定長バイトバッファです。
///
/// Internally this stores `[u8; N]`, so values can be handled efficiently on the stack.
/// 内部的には `[u8; N]` を保持しており、スタック上で効率的に処理されます。
///
/// It supports both string-oriented and byte-oriented operations.
/// 文字列としての操作と、バイト列としての操作の両方をサポートします。
///
/// # Generics
/// * `N`: The byte length of the buffer as a compile-time constant.
/// * `N`: バイト配列の長さを表すコンパイル時定数です。
///
/// # Examples
///
/// ```
/// use fixed_record::Fixed;
///
/// // Create a 10-byte buffer and write a string into it.
/// // 10バイトのバッファを作成し、文字列を書き込みます。
/// let mut name = Fixed::<10>::spaced();
/// name.write_bytes(b"Rust");
///
/// assert_eq!(name.as_bytes(), b"Rust      ");
/// assert_eq!(name.as_str().unwrap(), "Rust      ");
/// ```
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed<const N: usize> {
    /// The fixed-size array that stores the raw bytes.
    /// 内部データを保持する固定長配列です。
    pub(crate) buf: [u8; N],
}

impl<const N: usize> Fixed<N> {
    /// Copies the first `N` bytes from a byte slice into a new value.
    /// バイトスライスの先頭から `N` バイトをコピーして、新しいインスタンスを生成します。
    ///
    /// # Arguments
    /// * `src` - The source byte slice to copy from.
    /// * `src` - コピー元のバイトスライスです。
    ///
    /// # Errors
    /// Returns [`Error::TooShort`] when `src` is shorter than `N` bytes.
    /// `src` の長さが `N` 未満の場合、[`Error::TooShort`] を返します。
    ///
    /// # Examples
    ///
    /// ```
    /// use fixed_record::{Fixed, Error};
    ///
    /// // Success case.
    /// // 正常系です。
    /// let f = Fixed::<4>::from_slice(b"12345").unwrap();
    /// assert_eq!(f.as_bytes(), b"1234");
    ///
    /// // Error case: the input is too short.
    /// // 異常系: 入力が短すぎます。
    /// let err = Fixed::<4>::from_slice(b"123");
    /// assert!(matches!(err, Err(Error::TooShort)));
    /// ```
    pub fn from_slice(src: &[u8]) -> Result<Self, Error> {
        if src.len() < N {
            return Err(Error::TooShort);
        }
        let mut buf = [0u8; N];
        buf.copy_from_slice(&src[..N]);
        Ok(Self { buf })
    }

    /// Reads `N` bytes from `src` starting at `offset`.
    /// バイトスライスの指定された位置 `offset` から `N` バイトを読み取ります。
    ///
    /// # Arguments
    /// * `src` - The source byte slice to read from.
    /// * `src` - 読み取り元のバイトスライスです。
    /// * `offset` - The byte index where reading starts.
    /// * `offset` - 読み取り開始位置のバイトインデックスです。
    ///
    /// # Errors
    /// Returns [`Error::TooShort`] when `N` bytes are not available from `offset`.
    /// 指定された `offset` から `N` バイト分のデータが存在しない場合、[`Error::TooShort`] を返します。
    ///
    /// # Examples
    ///
    /// ```
    /// use fixed_record::Fixed;
    ///
    /// let raw_data = b"ID001NAME_YAMADA    ";
    /// // Extract 12 bytes starting at byte 5.
    /// // 5バイト目から12バイト分を抽出します。
    /// let name = Fixed::<12>::from_slice_at(raw_data, 5).unwrap();
    /// assert_eq!(name.as_str().unwrap(), "NAME_YAMADA ");
    /// ```
    pub fn from_slice_at(src: &[u8], offset: usize) -> Result<Self, Error> {
        let end = offset + N;
        if src.len() < end {
            return Err(Error::TooShort);
        }
        let mut buf = [0u8; N];
        buf.copy_from_slice(&src[offset..end]);
        Ok(Self { buf })
    }

    /// Borrows the stored bytes as a UTF-8 string.
    /// 保持しているバイト列を UTF-8 文字列として参照します。
    ///
    /// # Returns
    /// A string slice when the stored bytes are valid UTF-8.
    /// 有効な UTF-8 文字列のスライスです。
    ///
    /// # Errors
    /// Returns [`Error::Utf8Error`] when the stored bytes are not valid UTF-8.
    /// 内部データが有効な UTF-8 でない場合、[`Error::Utf8Error`] を返します。
    ///
    /// # Examples
    ///
    /// ```
    /// use fixed_record::Fixed;
    ///
    /// let f = Fixed::from(*b"Hello ");
    /// assert_eq!(f.as_str().unwrap(), "Hello ");
    ///
    /// // Invalid UTF-8 bytes fail.
    /// // 不正な UTF-8 バイト列はエラーになります。
    /// let f_bad = Fixed::from([0xFF, 0xFF]);
    /// assert!(f_bad.as_str().is_err());
    /// ```
    pub fn as_str(&self) -> Result<&str, Error> {
        std::str::from_utf8(&self.buf).map_err(|_| Error::Utf8Error)
    }

    /// Returns the internal buffer as a byte slice.
    /// 内部バッファをバイトスライスとして取得します。
    ///
    /// This is useful when callers need a zero-copy view of the data.
    /// ゼロコピーでデータを参照したい場合に便利です。
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Creates a value filled with `0x00` bytes.
    /// すべてのバイトが `0x00` で埋められたインスタンスを生成します。
    ///
    /// # Examples
    /// ```
    /// use fixed_record::Fixed;
    /// let f: Fixed<4> = Fixed::zeroed();
    /// assert_eq!(f.as_bytes(), &[0, 0, 0, 0]);
    /// ```
    pub const fn zeroed() -> Self {
        Self { buf: [0u8; N] }
    }

    /// Creates a value filled with `0x20` space bytes.
    /// すべてのバイトが `0x20` の半角スペースで埋められたインスタンスを生成します。
    ///
    /// This is useful for text fixed-width files that use space padding.
    /// テキスト形式の固定長ファイルで、空白埋めが必要な場合に便利です。
    ///
    /// # Examples
    /// ```
    /// use fixed_record::Fixed;
    /// let f: Fixed<3> = Fixed::spaced();
    /// assert_eq!(f.as_bytes(), b"   ");
    /// ```
    pub const fn spaced() -> Self {
        Self { buf: [b' '; N] }
    }

    /// Creates a value filled with the specified byte.
    /// すべてのバイトが指定された値で埋められたインスタンスを生成します。
    pub const fn filled(byte: u8) -> Self {
        Self { buf: [byte; N] }
    }

    /// Writes arbitrary bytes into the buffer.
    /// 任意のバイト列をバッファに書き込みます。
    ///
    /// # Behavior
    /// # 挙動
    /// - If `src` is longer than `N`, only the first `N` bytes are written and the rest is truncated.
    /// - `src` の長さが `N` より長い場合、先頭 `N` バイトのみを書き込み、残りは切り捨てます。
    /// - If `src` is shorter than `N`, only the leading bytes are replaced and the remaining bytes keep their previous values.
    /// - `src` の長さが `N` より短い場合、先頭部分だけを書き換え、残りのバイトは以前の値を維持します。
    ///
    /// # Arguments
    /// * `src` - The bytes to write.
    /// * `src` - 書き込むバイト列です。
    ///
    /// # Examples
    ///
    /// ```
    /// use fixed_record::Fixed;
    ///
    /// // Truncation.
    /// // 切り捨ての例です。
    /// let mut f = Fixed::<3>::spaced();
    /// f.write_bytes(b"ABCDEFG");
    /// assert_eq!(f.as_bytes(), b"ABC");
    ///
    /// // Partial overwrite: the remaining spaces are preserved.
    /// // 部分書き換えの例です。残りのスペースは維持されます。
    /// let mut f = Fixed::<5>::spaced();
    /// f.write_bytes(b"Hi");
    /// assert_eq!(f.as_bytes(), b"Hi   ");
    /// ```
    pub fn write_bytes(&mut self, src: &[u8]) {
        let len = src.len().min(N);
        self.buf[..len].copy_from_slice(&src[..len]);
    }

    /// Fills the internal buffer with the specified byte.
    /// 内部バッファを指定バイトで上書きします。
    pub fn fill(&mut self, byte: u8) {
        self.buf = [byte; N];
    }

    /// Fills the internal buffer with `0x00` bytes.
    /// 内部バッファをすべて `0x00` で上書きします。
    pub fn fill_zero(&mut self) {
        self.buf = [0u8; N];
    }

    /// Fills the internal buffer with `0x20` space bytes.
    /// 内部バッファをすべて半角スペース (`0x20`) で上書きします。
    pub fn fill_space(&mut self) {
        self.buf = [b' '; N];
    }
}

impl<const N: usize> fmt::Debug for Fixed<N> {
    /// Formats as a string when the bytes are valid UTF-8, otherwise as a byte array.
    /// UTF-8 として有効なら文字列形式で、そうでなければバイト配列形式で出力します。
    ///
    /// # Examples
    /// ```
    /// use fixed_record::Fixed;
    /// let f = Fixed::from(*b"ABC");
    /// assert_eq!(format!("{:?}", f), "Fixed(\"ABC\")");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_str() {
            Ok(s) => write!(f, "Fixed(\"{}\")", s),
            Err(_) => write!(f, "Fixed({:?})", &self.buf),
        }
    }
}

impl<const N: usize> fmt::Display for Fixed<N> {
    /// Displays the internal bytes directly as a string.
    /// 内部のバイト列を直接文字列として出力します。
    ///
    /// Invalid UTF-8 is displayed as `<?>`.
    /// 不正な UTF-8 が含まれる場合は `<?>` を表示します。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = std::str::from_utf8(&self.buf).unwrap_or("<?>");
        write!(f, "{}", s)
    }
}

impl<const N: usize> Default for Fixed<N> {
    /// Returns [`Fixed::zeroed()`].
    /// [`Fixed::zeroed()`] を返します。
    fn default() -> Self {
        Self::zeroed()
    }
}

impl<const N: usize> From<[u8; N]> for Fixed<N> {
    /// Converts directly from a fixed-size byte array.
    /// 固定長配列から直接変換します。
    fn from(buf: [u8; N]) -> Self {
        Self { buf }
    }
}

impl<const N: usize> From<&[u8; N]> for Fixed<N> {
    /// Copies from a fixed-size byte array reference.
    /// 固定長配列の参照からコピーして変換します。
    fn from(buf: &[u8; N]) -> Self {
        Self { buf: *buf }
    }
}
