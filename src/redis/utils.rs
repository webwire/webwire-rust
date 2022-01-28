use redis::ToRedisArgs;

pub(crate) struct RedisBytes(pub bytes::Bytes);

impl ToRedisArgs for RedisBytes {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg(&self.0)
    }
}
