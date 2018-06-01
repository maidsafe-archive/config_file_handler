// Copyright 2018 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use std::sync::{Mutex, Once, ONCE_INIT};

pub type GlobalMutex = Mutex<()>;

#[allow(unsafe_code)]
pub fn get_mutex<'a>() -> &'a GlobalMutex {
    static mut GLOBAL_MUTEX: *const GlobalMutex = 0 as *const GlobalMutex;
    static ONCE: Once = ONCE_INIT;

    unsafe {
        ONCE.call_once(|| {
            GLOBAL_MUTEX = Box::into_raw(Box::new(GlobalMutex::new(())));
        });

        &*GLOBAL_MUTEX
    }
}
