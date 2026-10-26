use sdl3::gpu::{
    BufferRegion, BufferUsageFlags, Device, TransferBufferLocation, TransferBufferUsage,
};

use crate::mesh::{Mesh, MeshData, VertexAttributes, VertexPosition};

pub struct MemAllocator<'a> {
    device: &'a Device,
}

impl<'a> MemAllocator<'a> {
    #[must_use]
    pub const fn new(device: &'a Device) -> Self {
        Self { device }
    }

    /// # Errors
    pub fn allocate_mesh(&self, mesh: &MeshData) -> anyhow::Result<Mesh> {
        let pos_bytes = bytemuck::cast_slice::<VertexPosition, u8>(&mesh.positions);
        let attr_bytes = bytemuck::cast_slice::<VertexAttributes, u8>(&mesh.attributes);
        let idx_bytes = bytemuck::cast_slice::<u16, u8>(&mesh.indices);

        let pos_size = pos_bytes.len();
        let attr_size = attr_bytes.len();
        let idx_size = idx_bytes.len();

        let pos_offset = 0;
        let attr_offset = (pos_offset + pos_size).next_multiple_of(4);
        let idx_offset = (attr_offset + attr_size).next_multiple_of(4);
        let total_packed_size: u32 = (idx_offset + idx_size).next_multiple_of(4).try_into()?;

        let position_buffer = self
            .device
            .create_buffer()
            .with_usage(BufferUsageFlags::VERTEX)
            .with_size(pos_size.try_into()?)
            .build()?;

        let attribute_buffer = self
            .device
            .create_buffer()
            .with_usage(BufferUsageFlags::VERTEX)
            .with_size(attr_size.try_into()?)
            .build()?;

        let index_buffer = self
            .device
            .create_buffer()
            .with_usage(BufferUsageFlags::INDEX)
            .with_size(idx_size.try_into()?)
            .build()?;

        let transfer_buffer = self
            .device
            .create_transfer_buffer()
            .with_usage(TransferBufferUsage::UPLOAD)
            .with_size(total_packed_size)
            .build()?;

        {
            let mut transfer_data = transfer_buffer.map::<u8>(self.device, false);
            let staging_slice = transfer_data.mem_mut();

            staging_slice[pos_offset..pos_offset + pos_size].copy_from_slice(pos_bytes);
            staging_slice[attr_offset..attr_offset + attr_size].copy_from_slice(attr_bytes);
            staging_slice[idx_offset..idx_offset + idx_size].copy_from_slice(idx_bytes);

            transfer_data.unmap();
        }

        let cmd_buf = self.device.acquire_command_buffer()?;
        let copy_pass = self.device.begin_copy_pass(&cmd_buf)?;

        // Upload Positions (Slot 0 source)
        copy_pass.upload_to_gpu_buffer(
            TransferBufferLocation::new()
                .with_transfer_buffer(&transfer_buffer)
                .with_offset(pos_offset.try_into()?),
            BufferRegion::new()
                .with_buffer(&position_buffer)
                .with_offset(0)
                .with_size(pos_size.try_into()?),
            false,
        );

        // Upload Attributes (Slot 1 source)
        copy_pass.upload_to_gpu_buffer(
            TransferBufferLocation::new()
                .with_transfer_buffer(&transfer_buffer)
                .with_offset(attr_offset.try_into()?),
            BufferRegion::new()
                .with_buffer(&attribute_buffer)
                .with_offset(0)
                .with_size(attr_size.try_into()?),
            false,
        );

        // Upload Indices
        copy_pass.upload_to_gpu_buffer(
            TransferBufferLocation::new()
                .with_transfer_buffer(&transfer_buffer)
                .with_offset(idx_offset.try_into()?),
            BufferRegion::new()
                .with_buffer(&index_buffer)
                .with_offset(0)
                .with_size(idx_size.try_into()?),
            false,
        );

        self.device.end_copy_pass(copy_pass);
        cmd_buf.submit()?;

        Ok(Mesh {
            position_buffer,
            attribute_buffer,
            index_buffer,
            index_count: idx_size.try_into()?,
        })
    }
}
