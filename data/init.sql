CREATE TABLE IF NOT EXISTS system_configs (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    key_name TEXT NOT NULL,
    value TEXT NOT NULL,
    is_encrypted BOOLEAN DEFAULT 0,
    version INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime')),
    UNIQUE(category, key_name)
);

CREATE TABLE IF NOT EXISTS models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    model_type TEXT NOT NULL,
    base_url TEXT,
    is_active BOOLEAN DEFAULT 1,
    health_status TEXT DEFAULT 'unknown',
    last_health_check TEXT,
    health_check_interval_seconds INTEGER DEFAULT 300,
    cost_per_token_input REAL DEFAULT 0,
    cost_per_token_output REAL DEFAULT 0,
    function_tags TEXT, -- 用逗号分隔字符串
    config TEXT,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime'))
);

-- Web管理界面需要的Provider表
CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,  -- ollama, ali, openai等
    display_name TEXT NOT NULL, -- 显示名称
    base_url TEXT,              -- 基础URL
    description TEXT,           -- 描述
    is_active BOOLEAN DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT DEFAULT (datetime('now', 'localtime'))
);

-- 插入默认的Provider数据
INSERT OR IGNORE INTO providers (id, name, display_name, description) VALUES 
    ('ollama', 'ollama', 'Ollama', '本地部署的开源大语言模型服务'),
    ('ali', 'ali', '阿里云通义千问', '阿里云提供的商业化大语言模型服务'),
    ('openai', 'openai', 'OpenAI', 'OpenAI提供的GPT系列模型'),
    ('zhipu', 'zhipu', '智谱AI', '智谱AI提供的GLM系列模型');

CREATE TABLE IF NOT EXISTS provider_key_pools (
    id TEXT PRIMARY KEY,
    provider_ TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    encrypted_key_value TEXT NOT NULL,
    is_active BOOLEAN DEFAULT 1,
    usage_count INTEGER DEFAULT 0,
    last_used_at TEXT,
    rate_limit_per_minute INTEGER,
    rate_limit_per_hour INTEGER,
    created_at TEXT DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS call_logs (
    id TEXT PRIMARY KEY,
    model_id TEXT,    
    status_code INTEGER NOT NULL,    
    total_duration INTEGER NOT NULL, -- in milliseconds
    tokens_output INTEGER DEFAULT 0,    
    error_message TEXT,
    created_at TEXT DEFAULT (datetime('now', 'localtime')),
    FOREIGN KEY(model_id) REFERENCES models(id)
);

CREATE TABLE IF NOT EXISTS metrics_snapshots (
    id TEXT PRIMARY KEY,
    snapshot_time TEXT NOT NULL,
    total_requests INTEGER,
    total_tokens_input INTEGER,
    total_tokens_output INTEGER,
    total_cost REAL,
    avg_latency_ms REAL,
    error_rate REAL,
    top_models TEXT,
    provider_stats TEXT
);

CREATE INDEX IF NOT EXISTS idx_models_provider_active ON models(provider, is_active);
CREATE INDEX IF NOT EXISTS idx_call_logs_created_at ON call_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_call_logs_model_id ON call_logs(model_id);