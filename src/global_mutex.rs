// Copyright 2016 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences").
//
// By contributing code to the SAFE Network Software, or to this project generally, you agree to be
// bound by the terms of the MaidSafe Contributor Agreement.  This, along with the Licenses can be
// found in the root directory of this project at LICENSE, COPYING and CONTRIBUTOR.
//
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.
//
// Please review the Licences for the specific language governing permissions and limitations
// relating to use of the SAFE Network Software.

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
        ONCE.call_once(|| { GLOBAL_MUTEX = Box::into_raw(Box::new(GlobalMutex::new(()))); });

        &*GLOBAL_MUTEX
    }
}
