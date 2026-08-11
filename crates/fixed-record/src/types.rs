use crate::error::Error;
use std::fmt;

/// コンパイル時にサイズ `N` が決定される固定長バイトバッファ。
///
/// 内部的には `[u8; N]` を保持しており、スタック上で効率的に処理されます。
/// 文字列としての操作と、バイト列としての操作の両方をサポートします。
///
/// # Generics
/// * `N`: バイト配列の長さ（コンパイル時定数）
///
/// # Examples
///
/// ```
/// use fixed_record::Fixed;
///
/// // 10バイトのバッファを作成し、文字列を書き込む
/// let mut name = Fixed::<10>::spaced();
/// name.write_bytes(b"Rust");
///
/// assert_eq!(name.as_bytes(), b"Rust      ");
/// assert_eq!(name.as_str().unwrap(), "Rust      ");
/// ```
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed<const N: usize> {
    /// 内部データを保持する固定長配列。
    pub(crate) buf: [u8; N],
}

impl<const N: usize> Fixed<N> {
    /// バイトスライスの先頭から `N` バイトをコピーして、新しいインスタンスを生成します。
    ///
    /// # Arguments
    /// * `src` - コピー元のバイトスライス。
    ///
    /// # Errors
    /// `src` の長さが `N` 未満の場合、[`Error::TooShort`] を返します。
    ///
    /// # Examples
    ///
    /// ```
    /// use fixed_record::{Fixed, Error};
    ///
    /// // 正常系
    /// let f = Fixed::<4>::from_slice(b"12345").unwrap();
    /// assert_eq!(f.as_bytes(), b"1234");
    ///
    /// // 異常系 (長さ不足)
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

    /// バイトスライスの指定された位置 `offset` から `N` バイトを読み取ります。
    ///
    /// # Arguments
    /// * `src` - 読み取り元のバイトスライス。
    /// * `offset` - 読み取り開始位置（インデックス）。
    ///
    /// # Errors
    /// 指定された `offset` から `N` バイト分のデータが存在しない場合、[`Error::TooShort`] を返します。
    ///
    /// # Examples
    ///
    /// ```
    /// use fixed_record::Fixed;
    ///
    /// let raw_data = b"ID001NAME_YAMADA    ";
    /// // 5文字目から12バイト分（"NAME_YAMADA"）を抽出
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

    /// 保持しているバイト列を UTF-8 文字列として参照します。
    ///
    /// # Returns
    /// 有効なUTF-8文字列のスライス。
    ///
    /// # Errors
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
    /// // 不正なUTF-8シーケンスの場合
    /// let f_bad = Fixed::from([0xFF, 0xFF]);
    /// assert!(f_bad.as_str().is_err());
    /// ```
    pub fn as_str(&self) -> Result<&str, Error> {
        std::str::from_utf8(&self.buf).map_err(|_| Error::Utf8Error)
    }

    /// 内部バッファをバイトスライスとして取得します。
    ///
    /// ゼロコピーでデータを参照したい場合に最適です。
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// すべてのバイトが `0x00` (Null) で埋められたインスタンスを生成します。
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

    /// すべてのバイトが `0x20` (半角スペース) で埋められたインスタンスを生成します。
    ///
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

    /// すべてのバイトが指定された値で埋められたインスタンスを生成します。
    pub const fn filled(byte: u8) -> Self {
        Self { buf: [byte; N] }
    }

    /// 任意のバイト列をバッファに書き込みます。
    ///
    /// # 挙動
    /// - `src` の長さが `N` より **長い** 場合：先頭 `N` バイトのみが書き込まれ、残りは切り捨てられます。
    /// - `src` の長さが `N` より **短い** 場合：`src` の分だけ前方から書き換えられ、**残りのバイトは以前の値を維持します**。
    ///
    /// # Arguments
    /// * `src` - 書き込むバイト列。
    ///
    /// # Examples
    ///
    /// ```
    /// use fixed_record::Fixed;
    ///
    /// // 切り捨ての例
    /// let mut f = Fixed::<3>::spaced();
    /// f.write_bytes(b"ABCDEFG");
    /// assert_eq!(f.as_bytes(), b"ABC");
    ///
    /// // 部分書き換えの例（残りのスペースは維持される）
    /// let mut f = Fixed::<5>::spaced();
    /// f.write_bytes(b"Hi");
    /// assert_eq!(f.as_bytes(), b"Hi   ");
    /// ```
    pub fn write_bytes(&mut self, src: &[u8]) {
        let len = src.len().min(N);
        self.buf[..len].copy_from_slice(&src[..len]);
    }

    /// 内部バッファを指定バイトで上書きします。
    pub fn fill(&mut self, byte: u8) {
        self.buf = [byte; N];
    }

    /// 内部バッファをすべて `0x00` で上書きします。
    pub fn fill_zero(&mut self) {
        self.buf = [0u8; N];
    }

    /// 内部バッファをすべて半角スペース (`0x20`) で上書きします。
    pub fn fill_space(&mut self) {
        self.buf = [b' '; N];
    }
}

impl<const N: usize> fmt::Debug for Fixed<N> {
    /// UTF-8として有効なら文字列形式で、そうでなければ16進数配列形式で出力します。
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
    /// 内部のバイト列を直接文字列として出力します。
    /// 不正なUTF-8が含まれる場合は `<?>` を表示します。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = std::str::from_utf8(&self.buf).unwrap_or("<?>");
        write!(f, "{}", s)
    }
}

impl<const N: usize> Default for Fixed<N> {
    /// [`Fixed::zeroed()`] を返します。
    fn default() -> Self {
        Self::zeroed()
    }
}

impl<const N: usize> From<[u8; N]> for Fixed<N> {
    /// 固定長配列から直接変換します。
    fn from(buf: [u8; N]) -> Self {
        Self { buf }
    }
}

impl<const N: usize> From<&[u8; N]> for Fixed<N> {
    /// 固定長配列の参照からコピーして変換します。
    fn from(buf: &[u8; N]) -> Self {
        Self { buf: *buf }
    }
}
