//! 错误类型别名

/// agent 命令执行结果：成功为输出文本，失败为错误信息
pub type CmdOutput = Result<String, String>;

/// teloxide handler 返回类型
pub type ResponseResult<T> = Result<T, teloxide::RequestError>;
