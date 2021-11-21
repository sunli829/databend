// Copyright 2020 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use common_exception::Result;
use common_meta_types::AuthType;
use common_planners::CreateUserPlan;
use common_planners::PlanNode;

use crate::sessions::DatabendQueryContextRef;
use crate::sql::statements::AnalyzableStatement;
use crate::sql::statements::AnalyzedResult;

#[derive(Debug, Clone, PartialEq)]
pub struct DfCreateUser {
    pub if_not_exists: bool,
    /// User name
    pub name: String,
    pub hostname: String,
    pub auth_type: AuthType,
    pub password: String,
}

#[async_trait::async_trait]
impl AnalyzableStatement for DfCreateUser {
    async fn analyze(&self, _ctx: DatabendQueryContextRef) -> Result<AnalyzedResult> {
        Ok(AnalyzedResult::SimpleQuery(PlanNode::CreateUser(
            CreateUserPlan {
                name: self.name.clone(),
                password: Vec::from(self.password.clone()),
                hostname: self.hostname.clone(),
                auth_type: self.auth_type.clone(),
            },
        )))
    }
}