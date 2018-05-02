// Copyright 2018 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use serde_json::Error as JsonError;
use std::env::VarError;
use std::io::Error as IoError;

quick_error! {
    /// Error types.
    #[derive(Debug)]
    pub enum Error {
        /// Wrapper for a `::std::env::VarError`
        Env(err: VarError) {
            description("Environment error")
            display("Environment error: {}", err)
            cause(err)
            from()
        }
        /// Wrapper for a `::std::io::Error`
        Io(err: IoError) {
            description("IO error")
            display("IO error: {}", err)
            cause(err)
            from()
        }
        /// Wrapper for a `::serde_json::Error`
        JsonParser(err: JsonError) {
            description("Json parse error")
            display("Json parse error: {}", err)
            cause(err)
            from()
        }
    }
}
