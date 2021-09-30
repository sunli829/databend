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

use std::task::Context;
use std::task::Poll;
use std::usize;

use common_datablocks::DataBlock;
use common_datavalues::prelude::*;
use common_exception::Result;
use common_streams::ProgressStream;
use futures::stream::Stream;

use crate::sessions::DatabendQueryContextRef;

#[derive(Debug, Clone)]
struct BlockRange {
    begin: u64,
    end: u64,
}

pub struct NumbersStream {
    ctx: DatabendQueryContextRef,
    schema: DataSchemaRef,
    block_index: usize,
    blocks: Vec<BlockRange>,
}

impl NumbersStream {
    pub fn try_create(
        ctx: DatabendQueryContextRef,
        schema: DataSchemaRef,
    ) -> Result<ProgressStream> {
        let stream = Box::pin(NumbersStream {
            ctx: ctx.clone(),
            schema,
            block_index: 0,
            blocks: vec![],
        });
        ProgressStream::try_create(stream, ctx.progress_callback()?)
    }

    #[inline]
    fn try_get_one_block(&mut self) -> Result<Option<DataBlock>> {
        if (self.block_index as usize) == self.blocks.len() {
            let partitions = self.ctx.try_get_partitions(1)?;
            if partitions.is_empty() {
                return Ok(None);
            }
            if partitions.len() == 1 && partitions[0].name.is_empty() {
                return Ok(None);
            }

            let block_size = self.ctx.get_settings().get_max_block_size()?;
            let mut blocks = Vec::with_capacity(partitions.len());
            for part in partitions {
                let names: Vec<_> = part.name.split('-').collect();
                let begin: u64 = names[1].parse()?;
                let end: u64 = names[2].parse()?;

                let diff = end - begin;
                let block_nums = diff / block_size;
                let block_remain = diff % block_size;

                if block_nums == 0 {
                    blocks.push(BlockRange { begin, end });
                } else {
                    for r in 0..block_nums {
                        let range_begin = begin + block_size * r;
                        let mut range_end = range_begin + block_size;
                        if r == (block_nums - 1) && block_remain > 0 {
                            range_end += block_remain;
                        }
                        blocks.push(BlockRange {
                            begin: range_begin,
                            end: range_end,
                        });
                    }
                }
            }
            self.blocks = blocks;
            self.block_index = 0;
        }

        let current = self.blocks[self.block_index].clone();
        self.block_index += 1;

        Ok(if current.begin == current.end {
            None
        } else {
            let size = (current.end - current.begin) as usize;
            let mut av = AlignedVec::with_capacity(size);

            unsafe { av.set_len(size) };

            av.as_mut_slice()
                .iter_mut()
                .enumerate()
                .for_each(|(idx, num)| {
                    *num = current.begin + idx as u64;
                });

            let series = DFUInt64Array::new_from_aligned_vec(av).into_series();
            let block = DataBlock::create_by_array(self.schema.clone(), vec![series]);
            Some(block)
        })
    }
}

impl Stream for NumbersStream {
    type Item = Result<DataBlock>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let block = self.try_get_one_block()?;

        Poll::Ready(block.map(Ok))
    }
}