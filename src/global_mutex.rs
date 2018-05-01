// Copyright 2018 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

// Without this mechanism the test cases in this crate have a race condition. File
// creation/deletion can only be protected by a global mutex if multiple threads are going for such
// operations.

use std::sync::{Mutex, ONCE_INIT, Once};

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
