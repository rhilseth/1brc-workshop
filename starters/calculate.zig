pub fn main(init: std.process.Init) !void {
    var file = try std.Io.Dir.cwd().openFile(init.io, "measurements.txt", .{});
    defer file.close(init.io);
    var results = std.StringHashMap(StationData).init(init.gpa);
    defer results.deinit();

    var read_buffer: [8192]u8 = undefined;
    var file_reader = file.readerStreaming(init.io, &read_buffer);
    var reader = &file_reader.interface;
    var line_buffer: [128]u8 = undefined;
    while (true) {
        var line = std.Io.Writer.fixed(&line_buffer);
        _ = reader.streamDelimiter(&line, '\n') catch break;
        _ = try reader.take(1);
        const key = std.mem.sliceTo(line.buffered(), ';');
        const val = try std.fmt.parseFloat(f64, line.buffered()[key.len + 1 ..]);
        var entry = try results.getOrPutAdapted(key, results.ctx);
        if (entry.found_existing) {
            entry.value_ptr.sum += val;
            entry.value_ptr.min = @min(entry.value_ptr.min, val);
            entry.value_ptr.max = @max(entry.value_ptr.max, val);
            entry.value_ptr.count += 1;
        } else {
            const alloc_key = try init.arena.allocator().dupe(u8, key);
            entry.key_ptr.* = alloc_key;
            entry.value_ptr.sum = val;
            entry.value_ptr.min = val;
            entry.value_ptr.max = val;
            entry.value_ptr.count = 1;
        }
    }

    var it = results.iterator();
    while (it.next()) |entry| {
        const v = entry.value_ptr;
        const avg = v.sum / @as(f64, @floatFromInt(v.count));
        std.debug.print("{s};{d:0.1};{d:0.1};{d:0.1}\n", .{
            entry.key_ptr.*,
            v.min,
            avg,
            v.max,
        });
    }
}

const StationData = struct {
    sum: f64,
    min: f64,
    max: f64,
    count: usize,
    pub fn format(self: StationData, comptime fmt: []const u8, options: std.fmt.FormatOptions, writer: anytype) !void {
        _ = fmt;
        _ = options;
        const mean = self.sum / @as(f64, @floatFromInt(self.count));
        try writer.print("{d:.2};{d:.2};{d:.2}", .{ self.min, mean, self.max });
    }
};

const std = @import("std");
