// The MIT License (MIT)
//
// Copyright (c) 2013 Jeremy Letang (letang.jeremy@gmail.com)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

/*!
# rust-sndfile

__Libsndfile__ bindings and wrappers for Rust.

__Libsndfile__ is a library for reading and writing files containing sampled
sound through one standard library interface.

website: [libsndfile](http://www.mega-nerd.com/libsndfile).

Libsndfile is a library designed to allow the reading and writing of many
different sampled sound file formats (such as MS Windows WAV and
the Apple/SGI AIFF format) through one standard library interface.

During read and write operations, formats are seamlessly converted between the
format the application program has requested or supplied and the file's data
format. The application programmer can remain blissfully
unaware of issues such as file endian-ness and data format

# Installation

You must install __libsndfile__ to build the binding. You can download it
directly from the [website](http://www.mega-nerd.com/libsndfile/#Download),
or with your favorite package management tool.

Then clone the __rust-sndfile__ repository and build it with rustpkg:

```Shell
rustpkg build sndfile
```

*/

#![allow(dead_code)]
#![warn(missing_docs)]

extern crate libc;

use std::path::Path;
use std::ptr;
use std::ffi::{CStr, CString};
use std::ops::Drop;
use std::str;

#[doc(hidden)]
mod libsndfile {
    #[cfg(any(target_os="macos", target_os="linux"))]
    #[link(name = "sndfile")]
    extern {}

    #[cfg(windows)]
    #[link(name = "sndfile-1")]
    extern {}
}

mod ffi;

/// The `SndInfo` structure is for passing data between the calling
/// function and the library when opening a file for reading or writing.
#[derive(Clone, PartialEq, PartialOrd, Debug, Copy)]
#[repr(C)]
pub struct SndInfo {
    /// The number of frames
    pub frames : i64,
    /// The sample rate
    pub samplerate : i32,
    /// The number of channels
    pub channels : i32,
    /// The format from enum FormatType
    pub format : i32,
    /// The sections
    pub sections : i32,
    /// Is the file seekable
    pub seekable : i32
}

/// Modes availables for the open function.
#[derive(Clone, PartialEq, PartialOrd, Debug, Copy)]
pub enum OpenMode {
    /// Read only mode
    Read    = ffi::SFM_READ as isize,
    /// Write only mode
    Write   = ffi::SFM_WRITE as isize,
    /// Read and Write mode
    ReadWrite    = ffi::SFM_RDWR as isize
}

/// Type of strings available for method `get_string()`
#[derive(Clone, PartialEq, PartialOrd, Debug, Copy)]
pub enum StringSoundType {
    /// Get the title of the audio content
    Title       = ffi::SF_STR_TITLE as isize,
    /// Get the copyright of the audio content
    Copyright   = ffi::SF_STR_COPYRIGHT as isize,
    /// Get the software name used to create the audio content
    Software    = ffi::SF_STR_SOFTWARE as isize,
    /// Get the artist of the audio content
    Artist      = ffi::SF_STR_ARTIST as isize,
    /// Get the comment on the audio file
    Comment     = ffi::SF_STR_COMMENT as isize,
    /// Get the date of creation
    Date        = ffi::SF_STR_DATE as isize,
    /// The name of the album
    Album       = ffi::SF_STR_ALBUM as isize,
    /// The licence of the content
    License     = ffi::SF_STR_LICENSE as isize,
    /// The track number of the audio content in an album
    TrackNumber = ffi::SF_STR_TRACKNUMBER as isize,
    /// The genre of the audio content
    Genre       = ffi::SF_STR_GENRE as isize
}

/// Types of error who can be return by API functions
#[derive(Clone, PartialEq, PartialOrd, Debug, Copy)]
pub enum SndFileError {
    /// The file format is not recognized
    UnrecognisedFormat,
    /// There is an internal system error
    SystemError,
    /// The file is malformed
    MalformedFile,
    /// The encoding of the file is not supported by sndfile
    UnsupportedEncoding,
    /// Any internal error code
    InternalError(i32)
}

impl SndFileError {
    /// Get a string representation of the error, as returned by libsndfile's
    /// sf_error_number
    pub fn desc(&self) -> String {
        let error_code = match *self {
            SndFileError::UnrecognisedFormat => ffi::SF_ERR_UNRECOGNISED_FORMAT,
            SndFileError::SystemError => ffi::SF_ERR_SYSTEM,
            SndFileError::MalformedFile => ffi::SF_ERR_MALFORMED_FILE,
            SndFileError::UnsupportedEncoding => ffi::SF_ERR_UNSUPPORTED_ENCODING,
            SndFileError::InternalError(err) => err
        };
        unsafe {
            str::from_utf8_unchecked(CStr::from_ptr(
                ffi::sf_error_number(error_code)).to_bytes()).to_string()
        }
    }

    fn from_code(code: i32) -> Option<SndFileError> {
        match code {
            ffi::SF_ERR_NO_ERROR => None,
            ffi::SF_ERR_UNRECOGNISED_FORMAT => Some(SndFileError::UnrecognisedFormat),
            ffi::SF_ERR_SYSTEM => Some(SndFileError::SystemError),
            ffi::SF_ERR_MALFORMED_FILE => Some(SndFileError::MalformedFile),
            ffi::SF_ERR_UNSUPPORTED_ENCODING => Some(SndFileError::UnsupportedEncoding),
            _ => Some(SndFileError::InternalError(code))
        }
    }

    fn code_to_result<T>(code: i32, ok: T) -> SndFileResult<T> {
        match SndFileError::from_code(code) {
            Some(err) => Err(err),
            None => Ok(ok)
        }
    }
}

/// Type alias for a `Result` with `SndFileError`
pub type SndFileResult<T> = Result<T, SndFileError>;

/// Enum to set the offset with method seek
#[derive(Clone, PartialEq, PartialOrd, Debug, Copy)]
pub enum SeekMode {
    /// The offset is set to the start of the audio data plus offset (multichannel) frames.
    SeekSet = ffi::SEEK_SET as isize,
    /// The offset is set to its current location plus offset (multichannel) frames.
    SeekCur = ffi::SEEK_CUR as isize,
    /// The offset is set to the end of the data plus offset (multichannel) frames.
    SeekEnd = ffi::SEEK_END as isize
}

/// Enum who contains the list of the supported audio format
#[derive(Clone, PartialEq, PartialOrd, Debug, Copy)]
pub enum FormatType {
    /// Microsoft WAV format (little endian)
    FormatWav = ffi::SF_FORMAT_WAV as isize,
    /// Apple/SGI AIFF format (big endian)
    FormatAiff = ffi::SF_FORMAT_AIFF as isize,
    /// Sun/NeXT AU format (big endian)
    FormatAu = ffi::SF_FORMAT_AU as isize,
    /// RAW PCM data
    FormatRaw = ffi::SF_FORMAT_RAW as isize,
    /// Ensoniq PARIS file format
    FormatPaf = ffi::SF_FORMAT_PAF as isize,
    /// Amiga IFF / SVX8 / SV16 format
    FormatSvx = ffi::SF_FORMAT_SVX as isize,
    /// Sphere NIST format
    FormatNist = ffi::SF_FORMAT_NIST as isize,
    /// VOC files
    FormatVoc = ffi::SF_FORMAT_VOC as isize,
    /// Berkeley/IRCAM/CARL
    FormatIrcam = ffi::SF_FORMAT_IRCAM as isize,
    /// Sonic Foundry's 64 bit RIFF/WAV
    FormatW64 = ffi::SF_FORMAT_W64 as isize,
    /// Matlab (tm) V4.2 / GNU Octave 2.0
    FormatMat4 = ffi::SF_FORMAT_MAT4 as isize,
    /// Matlab (tm) V5.0 / GNU Octave 2.1
    FormatMat5 = ffi::SF_FORMAT_MAT5 as isize,
    /// Portable Voice Format
    FormatPvf = ffi::SF_FORMAT_PVF as isize,
    /// Fasttracker 2 Extended Instrument
    FormatXi = ffi::SF_FORMAT_XI as isize,
    /// HMM Tool Kit format
    FormatHtk = ffi::SF_FORMAT_HTK as isize,
    /// Midi Sample Dump Standard
    FormatSds = ffi::SF_FORMAT_SDS as isize,
    /// Audio Visual Research
    FormatAvr = ffi::SF_FORMAT_AVR as isize,
    /// MS WAVE with WAVEFORMATEX
    FormatWavex = ffi::SF_FORMAT_WAVEX as isize,
    /// Sound Designer 2
    FormatSd2 = ffi::SF_FORMAT_SD2 as isize,
    /// FLAC lossless file format
    FormatFlac = ffi::SF_FORMAT_FLAC as isize,
    /// Core Audio File format
    FormatCaf = ffi::SF_FORMAT_CAF as isize,
    /// Psion WVE format
    FormatWve = ffi::SF_FORMAT_WVE as isize,
    /// Xiph OGG container
    FormatOgg = ffi::SF_FORMAT_OGG as isize,
    /// Akai MPC 2000 sampler
    FormatMpc2k = ffi::SF_FORMAT_MPC2K as isize,
    /// RF64 WAV file
    FormatRf64 = ffi::SF_FORMAT_RF64 as isize,
    /// Signed 8 bit data
    FormatPcmS8 = ffi::SF_FORMAT_PCM_S8 as isize,
    /// Signed 16 bit data
    FormatPcm16 = ffi::SF_FORMAT_PCM_16 as isize,
    /// Signed 24 bit data
    FormatPcm24 = ffi::SF_FORMAT_PCM_24 as isize,
    /// Signed 32 bit data
    FormatPcm32 = ffi::SF_FORMAT_PCM_32 as isize,
    /// Unsigned 8 bit data (WAV and RAW only)
    FormatPcmU8 = ffi::SF_FORMAT_PCM_U8 as isize,
    /// 32 bit float data
    FormatFloat = ffi::SF_FORMAT_FLOAT as isize,
    /// 64 bit float data
    FormatDouble = ffi::SF_FORMAT_DOUBLE as isize,
    /// U-Law encoded
    FormatUlaw = ffi::SF_FORMAT_ULAW as isize,
    /// A-Law encoded
    FormatAlaw = ffi::SF_FORMAT_ALAW as isize,
    /// IMA ADPCM
    FormatImaAdpcm = ffi::SF_FORMAT_IMA_ADPCM as isize,
    /// Microsoft ADPCM
    FormatApcm = ffi::SF_FORMAT_MS_ADPCM  as isize,
    /// GSM 6.10 encoding
    FormatGsm610 = ffi::SF_FORMAT_GSM610 as isize,
    /// Oki Dialogic ADPCM encoding
    FormatVoxAdpcm = ffi::SF_FORMAT_VOX_ADPCM as isize,
    /// 32kbs G721 ADPCM encoding
    FormatG72132 = ffi::SF_FORMAT_G721_32 as isize,
    /// 24kbs G723 ADPCM encoding
    FormatG72324 = ffi::SF_FORMAT_G723_24 as isize,
    /// 40kbs G723 ADPCM encoding
    FormatG72340 = ffi::SF_FORMAT_G723_40 as isize,
    /// 12 bit Delta Width Variable Word encoding
    FormatDww12 = ffi::SF_FORMAT_DWVW_12 as isize,
    /// 16 bit Delta Width Variable Word encoding
    FormatDww16 = ffi::SF_FORMAT_DWVW_16 as isize,
    /// 24 bit Delta Width Variable Word encoding
    FormatDww24 = ffi::SF_FORMAT_DWVW_24 as isize,
    /// N bit Delta Width Variable Word encoding
    FormatDwwN = ffi::SF_FORMAT_DWVW_N as isize,
    /// 8 bit differential PCM (XI only)
    FormatDpcm8 = ffi::SF_FORMAT_DPCM_8 as isize,
    /// 16 bit differential PCM (XI only)
    FormatDpcm16 = ffi::SF_FORMAT_DPCM_16 as isize,
    /// Xiph Vorbis encoding
    FormatVorbis = ffi::SF_FORMAT_VORBIS as isize,
    /// Default file endian-ness
    EndianFile = ffi::SF_ENDIAN_FILE as isize,
    /// Force little endian-ness
    EndianLittle = ffi::SF_ENDIAN_LITTLE as isize,
    /// Force big endian-ness
    EndianBig = ffi::SF_ENDIAN_BIG as isize,
    /// Force CPU endian-ness
    EndianCpu = ffi::SF_ENDIAN_CPU as isize,
    /// Sub mask
    FormatSubMask = ffi::SF_FORMAT_SUBMASK as isize,
    /// Type mask
    FormatTypeMask = ffi::SF_FORMAT_TYPEMASK as isize,
}

/// `SndFile` object, used to load/store sound from a file path or an fd.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct SndFile {
    handle : *mut ffi::SNDFILE,
    info : SndInfo
}

unsafe impl Send for SndFile {}

impl Drop for SndFile {
    fn drop(&mut self) {
        self.close().unwrap();
    }
}

impl SndFile {
    /**
     * Construct SndFile object with the path to the music and a mode to open it.
     *
     * # Arguments
     * * `path` - The path to load the music
     * * `mode` - The mode to open the music
     *
     * Return Ok() containing the SndFile on success, a string representation
     * of the error otherwise.
     */
    pub fn new(path : &Path, mode : OpenMode) -> SndFileResult<SndFile> {
        let mut info : SndInfo = SndInfo {
            frames : 0,
            samplerate : 0,
            channels : 0,
            format : 0,
            sections : 0,
            seekable : 0
        };
        let c_path = path.to_str().unwrap().to_string();
        let cstr = CString::new(c_path).unwrap().as_ptr();
        let tmp_sndfile = unsafe {ffi::sf_open(cstr, mode as i32, &mut info as *mut SndInfo) };
        if tmp_sndfile.is_null() {
            Err(SndFileError::from_code(unsafe { ffi::sf_error(ptr::null_mut())})
                .expect("expected error from sf_error, got no error"))
        } else {
            Ok(SndFile {
                handle :    tmp_sndfile,
                info :      info
            })
        }
    }

    /**
     * Construct SndFile object with the fd of the file containing the music
     * and a mode to open it.
     *
     * # Arguments
     * * `fd` - The fd to load the music
     * * `mode` - The mode to open the music
     * * `close_desc` - Should SndFile close the fd at exit?
     *
     * Return Ok() containing the SndFile on success, a string representation of
     * the error otherwise.
     */
    pub fn new_with_fd(fd : i32,
                       mode : OpenMode,
                       close_desc : bool) -> SndFileResult<SndFile> {
        let info : SndInfo = SndInfo {
            frames : 0,
            samplerate : 0,
            channels : 0,
            format : 0,
            sections : 0,
            seekable : 0
        };
        let tmp_sndfile = unsafe {
            ffi::sf_open_fd(fd, mode as i32, &info,
                            if close_desc {
                                ffi::SF_TRUE
                            } else {
                                ffi::SF_FALSE
                            })
        };
        if tmp_sndfile.is_null() {
            Err(SndFileError::from_code(unsafe {
                ffi::sf_error(ptr::null_mut())
            }).expect("expected error from sf_error, got no error"))
        } else {
            Ok(SndFile {
                handle :    tmp_sndfile,
                info :      info
            })
        }
    }

    /// Return the SndInfo struct of the current music.
    pub fn get_sndinfo(&self) -> SndInfo {
        self.info
    }

    /**
     * Retrieve a tag contained by the music.
     *
     * # Argument
     * * `string_type` - The type of the tag to retrieve
     *
     * Return Some(String) if the tag is found, None otherwise.
     */
    pub fn get_string(&self, string_type : StringSoundType) -> Option<String> {
        let c_string = unsafe {
            ffi::sf_get_string(self.handle, string_type as i32)
        };
        if c_string.is_null() {
            None
        } else {
            Some(unsafe {
                str::from_utf8_unchecked(CStr::from_ptr(
                    c_string).to_bytes()).to_string()
            })
        }
    }

    /**
     * Set a tag on the music file.
     *
     * # Arguments
     * * `string_type` - The type of the tag to set
     * * `string` - The string to set.
     *
     * Return () on success, Err otherwise
     */
    pub fn set_string(&mut self,
                      string_type : StringSoundType,
                      string : &str) -> SndFileResult<()> {
        let error_code = unsafe {
            let c_str = string.as_ptr() as *const i8;
            ffi::sf_set_string(self.handle, string_type as i32, c_str)
        };
        SndFileError::code_to_result(error_code, ())
    }

    /**
     * Check if the format of the SndInfo struct is valid.
     *
     * # Argument
     * * `info` - The SndInfo struct to test
     *
     * Return true if the struct is valid, false otherwise.
     */
    pub fn check_format(info : &SndInfo) -> bool {
        match unsafe {ffi::sf_format_check(info) } {
            ffi::SF_TRUE    => true,
            ffi::SF_FALSE   => false,
            _               => unreachable!()
        }
    }


    /**
     * Close the SndFile object.
     *
     * This function must be called before the exist of the program to destroy
     * all the resources.
     *
     * Return NoError if destruction success, an other error code otherwise.
     */
    fn close(&self) -> SndFileResult<()> {
        let error_code = unsafe { ffi::sf_close(self.handle) };
        SndFileError::code_to_result(error_code, ())
    }

    /**
     * If the file is opened Write or ReadWrite, call the operating system's
     * function to force the writing of all file cache buffers to disk.
     * If the file is opened Read no action is taken.
     * If the file is opened Read no action is taken.
     */
    pub fn write_sync(&mut self) {
        unsafe {
            ffi::sf_write_sync(self.handle)
        }
    }

    /**
     * Move in the audio file
     *
     * The non audio data are ignored
     *
     * # Arguments
     * * `frames` - The position to move in frames
     * * `whence` - The seek mode from the enum SeekMode
     */
    pub fn seek(&mut self, frames : i64, whence : SeekMode) -> i64 {
        unsafe {
            ffi::sf_seek(self.handle, frames, whence as i32)
        }
    }

    /**
     * Read items of type i16
     *
     * # Arguments
     * * `array` - The array to fill with the items.
     * * `items` - The max capacity of the array.
     *
     * Return the count of items.
     */
    pub fn read_i16<'r>(&'r mut self,
                        array : &'r mut [i16],
                        items : i64) -> i64 {
        unsafe {
            ffi::sf_read_short(self.handle, array.as_mut_ptr(), items)
        }
    }

    /**
     * Read items of type i32
     *
     * # Arguments
     * * `array` - The array to fill with the items.
     * * `items` - The max capacity of the array.
     *
     * Return the count of items.
     */
    pub fn read_i32<'r>(&'r mut self,
                        array : &'r mut [i32],
                        items : i64) -> i64 {
        unsafe {
            ffi::sf_read_int(self.handle, array.as_mut_ptr(), items)
        }
    }

    /**
     * Read items of type f32
     *
     * # Arguments
     * * `array` - The array to fill with the items.
     * * `items` - The max capacity of the array.
     *
     * Return the count of items.
     */
    pub fn read_f32<'r>(&'r mut self,
                        array : &'r mut [f32],
                        items : i64) -> i64 {
        unsafe {
            ffi::sf_read_float(self.handle, array.as_mut_ptr(), items)
        }
    }

    /**
     * Read items of type f64
     *
     * # Arguments
     * * `array` - The array to fill with the items.
     * * `items` - The max capacity of the array.
     *
     * Return the count of items.
     */
    pub fn read_f64<'r>(&'r mut self,
                        array : &'r mut [f64],
                        items : i64) -> i64 {
        unsafe {
            ffi::sf_read_double(self.handle, array.as_mut_ptr(), items)
        }
    }

    /**
     * Read frames of type i16
     *
     * # Arguments
     * * `array` - The array to fill with the frames.
     * * `items` - The max capacity of the array.
     *
     * Return the count of frames.
     */
    pub fn readf_i16<'r>(&'r mut self,
                         array : &'r mut [i16],
                         frames : i64) -> i64 {
        unsafe {
            ffi::sf_readf_short(self.handle, array.as_mut_ptr(), frames)
        }
    }

    /**
     * Read frames of type i32
     *
     * # Arguments
     * * `array` - The array to fill with the frames.
     * * `items` - The max capacity of the array.
     *
     * Return the count of frames.
     */
    pub fn readf_i32<'r>(&'r mut self,
                         array : &'r mut [i32],
                         frames : i64) -> i64 {
        unsafe {
            ffi::sf_readf_int(self.handle, array.as_mut_ptr(), frames)
        }
    }

    /**
     * Read frames of type f32
     *
     * # Arguments
     * * `array` - The array to fill with the frames.
     * * `items` - The max capacity of the array.
     *
     * Return the count of frames.
     */
    pub fn readf_f32<'r>(&'r mut self,
                         array : &'r mut [f32],
                         frames : i64) -> i64 {
        unsafe {
            ffi::sf_readf_float(self.handle, array.as_mut_ptr(), frames)
        }
    }

    /**
     * Read frames of type f64
     *
     * # Arguments
     * * `array` - The array to fill with the frames.
     * * `items` - The max capacity of the array.
     *
     * Return the count of frames.
     */
    pub fn readf_f64<'r>(&'r mut self,
                         array : &'r mut [f64],
                         frames : i64) -> i64 {
        unsafe {
            ffi::sf_readf_double(self.handle, array.as_mut_ptr(), frames)
        }
    }

    /**
     * Write items of type i16
     *
     * # Arguments
     * * `array` - The array of items to write.
     * * `items` - The number of items to write.
     *
     * Return the count of wrote items.
     */
    pub fn write_i16<'r>(&'r mut self,
                         array : &'r mut [i16],
                         items : i64) -> i64 {
        unsafe {
            ffi::sf_write_short(self.handle, array.as_mut_ptr(), items)
        }
    }

    /**
     * Write items of type i32
     *
     * # Arguments
     * * `array` - The array of items to write.
     * * `items` - The number of items to write.
     *
     * Return the count of wrote items.
     */
    pub fn write_i32<'r>(&'r mut self,
                         array : &'r mut [i32],
                         items : i64) -> i64 {
        unsafe {
            ffi::sf_write_int(self.handle, array.as_mut_ptr(), items)
        }
    }

    /**
     * Write items of type f32
     *
     * # Arguments
     * * `array` - The array of items to write.
     * * `items` - The number of items to write.
     *
     * Return the count of wrote items.
     */
    pub fn write_f32<'r>(&'r mut self,
                         array : &'r mut [f32],
                         items : i64) -> i64 {
        unsafe {
            ffi::sf_write_float(self.handle, array.as_mut_ptr(), items)
        }
    }

    /**
     * Write items of type f64
     *
     * # Arguments
     * * `array` - The array of items to write.
     * * `items` - The number of items to write.
     *
     * Return the count of wrote items.
     */
    pub fn write_f64<'r>(&'r mut self,
                         array : &'r mut [f64],
                         items : i64) -> i64 {
        unsafe {
            ffi::sf_write_double(self.handle, array.as_mut_ptr(), items)
        }
    }

    /**
     * Write frames of type i16
     *
     * # Arguments
     * * `array` - The array of frames to write.
     * * `items` - The number of frames to write.
     *
     * Return the count of wrote frames.
     */
    pub fn writef_i16<'r>(&'r mut self,
                          array : &'r mut [i16],
                          frames : i64) -> i64 {
        unsafe {
            ffi::sf_writef_short(self.handle, array.as_mut_ptr(), frames)
        }
    }

    /**
     * Write frames of type i32
     *
     * # Arguments
     * * `array` - The array of frames to write.
     * * `items` - The number of frames to write.
     *
     * Return the count of wrote frames.
     */
    pub fn writef_i32<'r>(&'r mut self,
                          array : &'r mut [i32],
                          frames : i64) -> i64 {
        unsafe {
            ffi::sf_writef_int(self.handle, array.as_mut_ptr(), frames)
        }
    }

    /**
     * Write frames of type f32
     *
     * # Arguments
     * * `array` - The array of frames to write.
     * * `items` - The number of frames to write.
     *
     * Return the count of wrote frames.
     */
    pub fn writef_f32<'r>(&'r mut self,
                          array : &'r mut [f32],
                          frames : i64) -> i64 {
        unsafe {
            ffi::sf_writef_float(self.handle, array.as_mut_ptr(), frames)
        }
    }

    /**
     * Write frames of type f64
     *
     * # Arguments
     * * `array` - The array of frames to write.
     * * `items` - The number of frames to write.
     *
     * Return the count of wrote frames.
     */
    pub fn writef_f64<'r>(&'r mut self,
                          array : &'r mut [f64],
                          frames : i64) -> i64 {
        unsafe {
            ffi::sf_writef_double(self.handle, array.as_mut_ptr(), frames)
        }
    }

    /**
     * Get the last error if one exists or `None` if there has not been an
     * error.
     */
    pub fn error(&self) -> Option<SndFileError> {
        SndFileError::from_code(unsafe {
            ffi::sf_error(self.handle)
        })
    }
}
