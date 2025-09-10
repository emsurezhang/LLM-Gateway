// Global variables
let providers = [];
let models = [];
let modelTemplates = {};
let callLogs = [];
let callLogStats = {};
let currentPage = 'providers';
let currentCallLogPage = 1;
let callLogPageSize = 100;
let totalCallLogPages = 1;
let errorOnlyFilter = false;

// Initialize application
document.addEventListener('DOMContentLoaded', function() {
    initNavigation();
    initResponsive();
    initForms();
    initSearch();
    initCallLogsControls();
    
    // 检测当前激活的页面
    const activePanel = document.querySelector('.page-panel.active');
    if (activePanel) {
        currentPage = activePanel.id;
    }
    
    // 加载当前激活页面的数据
    loadPageData(currentPage);
});

// Navigation functions
function initNavigation() {
    const navLinks = document.querySelectorAll('.nav-link');
    const pagePanels = document.querySelectorAll('.page-panel');

    navLinks.forEach(link => {
        link.addEventListener('click', function(e) {
            e.preventDefault();
            const pageId = this.getAttribute('data-page');
            console.log('Navigation clicked:', pageId);
            
            // Update navigation state
            navLinks.forEach(l => l.classList.remove('active'));
            this.classList.add('active');
            
            // Update page display
            pagePanels.forEach(panel => {
                panel.classList.remove('active');
            });
            document.getElementById(pageId).classList.add('active');
            
            // Update breadcrumb
            updateBreadcrumb(pageId);
            
            // Record current page
            currentPage = pageId;
            
            // Load data based on page
            loadPageData(pageId);
            
            // Auto close menu on mobile
            if (window.innerWidth <= 768) {
                closeMobileMenu();
            }
        });
    });
}

// 更新面包屑
function updateBreadcrumb(pageId) {
    const pageNames = {
        'providers': '提供商管理',
        'models': '模型管理',
        'call-logs': '调用日志'
    };
    
    const element = document.getElementById('currentPageName');
    if (element) {
        element.textContent = pageNames[pageId] || pageId;
    }
}

// 加载页面数据
function loadPageData(pageId) {
    console.log('Loading page data for:', pageId);
    switch(pageId) {
        case 'providers':
            loadProviders();
            break;
        case 'models':
            loadModels();
            break;
        case 'call-logs':
            console.log('Loading call logs...');
            loadCallLogs();
            loadCallLogStats();
            break;
    }
}

// 响应式处理
function initResponsive() {
    // 检测屏幕尺寸变化
    window.addEventListener('resize', function() {
        if (window.innerWidth <= 768) {
            document.getElementById('mobileMenuBtn').style.display = 'block';
        } else {
            document.getElementById('mobileMenuBtn').style.display = 'none';
            closeMobileMenu();
        }
    });
    
    // 初始化移动端按钮显示状态
    if (window.innerWidth <= 768) {
        document.getElementById('mobileMenuBtn').style.display = 'block';
    }
}

// 搜索功能
function initSearch() {
    // 提供商搜索
    const providerSearch = document.getElementById('providerSearch');
    if (providerSearch) {
        providerSearch.addEventListener('input', function() {
            filterProviders(this.value);
        });
    }
    
    // 模型搜索
    const modelSearch = document.getElementById('modelSearch');
    if (modelSearch) {
        modelSearch.addEventListener('input', function() {
            filterModels(this.value);
        });
    }
}

// 过滤提供商
function filterProviders(searchTerm) {
    const providerCards = document.querySelectorAll('.provider-card');
    providerCards.forEach(card => {
        const text = card.textContent.toLowerCase();
        const isVisible = text.includes(searchTerm.toLowerCase());
        card.style.display = isVisible ? 'block' : 'none';
    });
}

// 过滤模型
function filterModels(searchTerm) {
    const modelRows = document.querySelectorAll('#models-table-body tr');
    modelRows.forEach(row => {
        const text = row.textContent.toLowerCase();
        const isVisible = text.includes(searchTerm.toLowerCase());
        row.style.display = isVisible ? '' : 'none';
    });
}

// 侧边栏控制
function toggleSidebar() {
    const sidebar = document.getElementById('sidebar');
    const toggleIcon = document.getElementById('toggleIcon');
    
    sidebar.classList.toggle('collapsed');
    toggleIcon.textContent = sidebar.classList.contains('collapsed') ? '→' : '←';
}

// 移动端菜单控制
function openMobileMenu() {
    const sidebar = document.getElementById('sidebar');
    const overlay = document.getElementById('mobileOverlay');
    
    sidebar.classList.add('mobile-open');
    overlay.classList.add('active');
}

function closeMobileMenu() {
    const sidebar = document.getElementById('sidebar');
    const overlay = document.getElementById('mobileOverlay');
    
    sidebar.classList.remove('mobile-open');
    overlay.classList.remove('active');
}

// 主题切换
function toggleTheme() {
    // 这里可以实现主题切换逻辑
    console.log('切换主题功能待实现');
}

// API调用函数
async function apiCall(url, options = {}) {
    try {
        const response = await fetch(url, {
            headers: {
                'Content-Type': 'application/json',
                ...options.headers
            },
            ...options
        });
        
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        return await response.json();
    } catch (error) {
        console.error('API调用失败:', error);
        throw error;
    }
}

// Load Providers
async function loadProviders() {
    const loadingEl = document.getElementById('providers-loading');
    const errorEl = document.getElementById('providers-error');
    const contentEl = document.getElementById('providers-content');
    const gridEl = document.getElementById('providers-grid');

    try {
        loadingEl.style.display = 'block';
        errorEl.style.display = 'none';
        contentEl.style.display = 'none';

        const data = await apiCall('/api/providers');
        providers = data;

        // 渲染Provider卡片
        gridEl.innerHTML = data.map(provider => `
            <div class="provider-card">
                <div class="provider-header">
                    <div>
                        <div class="provider-name">${escapeHtml(provider.display_name)}</div>
                        <div class="provider-type">${escapeHtml(provider.name)}</div>
                    </div>
                    <div class="status-badge ${provider.is_active ? 'status-active' : 'status-inactive'}">
                        ${provider.is_active ? '正常' : '停用'}
                    </div>
                </div>
                
                ${provider.description ? `<div style="color: #666; margin-bottom: 1rem; font-size: 0.9rem;">${escapeHtml(provider.description)}</div>` : ''}
                
                ${provider.base_url ? `<div style="font-size: 0.9rem; color: #666; margin-bottom: 1rem;">
                    <strong>Base URL:</strong> ${escapeHtml(provider.base_url)}
                </div>` : ''}
                
                <div class="provider-stats">
                    <div class="stat-item">
                        <div class="stat-value">${provider.model_count}</div>
                        <div class="stat-label">Models</div>
                    </div>
                    <div style="display: flex; gap: 0.5rem; flex-wrap: wrap;">
                        <button class="btn btn-secondary" onclick="editProvider('${provider.id}')" style="padding: 0.25rem 0.5rem; font-size: 0.8rem;">
                            编辑
                        </button>
                        <button class="btn btn-primary" onclick="manageApiKeys('${provider.id}', '${escapeHtml(provider.display_name)}')" style="padding: 0.25rem 0.5rem; font-size: 0.8rem;">
                            API Keys
                        </button>
                        <button class="btn btn-danger" onclick="deleteProvider('${provider.id}')" style="padding: 0.25rem 0.5rem; font-size: 0.8rem;">
                            删除
                        </button>
                    </div>
                </div>
            </div>
        `).join('');

        loadingEl.style.display = 'none';
        contentEl.style.display = 'block';
    } catch (error) {
        loadingEl.style.display = 'none';
        errorEl.textContent = '加载失败: ' + error.message;
        errorEl.style.display = 'block';
    }
}

// 加载Models
async function loadModels() {
    const loadingEl = document.getElementById('models-loading');
    const errorEl = document.getElementById('models-error');
    const contentEl = document.getElementById('models-content');
    const tableBodyEl = document.getElementById('models-table-body');

    try {
        loadingEl.style.display = 'block';
        errorEl.style.display = 'none';
        contentEl.style.display = 'none';

        const data = await apiCall('/api/models');
        models = data;

        // 渲染Model表格
        tableBodyEl.innerHTML = data.map(model => `
            <tr>
                <td>${escapeHtml(model.name)}</td>
                <td>${escapeHtml(model.provider_name)}</td>
                <td>${model.model_type.toUpperCase()}</td>
                <td>
                    <span class="status-badge ${model.is_active ? 'status-active' : 'status-inactive'}">
                        ${model.is_active ? '正常' : '停用'}
                    </span>
                </td>
                <td>${model.cost_per_token_input || 'N/A'}</td>
                <td>${model.cost_per_token_output || 'N/A'}</td>
                <td>
                    <button class="btn btn-secondary" onclick="editModel('${model.id}')" style="padding: 0.25rem 0.5rem; font-size: 0.8rem; margin-right: 0.5rem;">
                        编辑
                    </button>
                    <button class="btn btn-danger" onclick="deleteModel('${model.id}')" style="padding: 0.25rem 0.5rem; font-size: 0.8rem;">
                        删除
                    </button>
                </td>
            </tr>
        `).join('');

        loadingEl.style.display = 'none';
        contentEl.style.display = 'block';
    } catch (error) {
        loadingEl.style.display = 'none';
        errorEl.textContent = '加载失败: ' + error.message;
        errorEl.style.display = 'block';
    }
}

// 显示添加Provider模态框
function showAddProviderModal() {
    document.getElementById('add-provider-modal').classList.add('active');
}

// 显示添加Model模态框
async function showAddModelModal() {
    // 加载Provider选项
    const providerSelect = document.getElementById('model-provider');
    try {
        const providerSummary = await apiCall('/api/providers/summary');
        providerSelect.innerHTML = '<option value="">Please select...</option>' + 
            providerSummary.map(p => `<option value="${p.id}">${escapeHtml(p.display_name)}</option>`).join('');
    } catch (error) {
        console.error('Failed to load providers:', error);
        providerSelect.innerHTML = '<option value="">Loading failed</option>';
    }
    
    document.getElementById('add-model-modal').classList.add('active');
}

// 加载模型模板
async function loadModelTemplates() {
    const providerId = document.getElementById('model-provider').value;
    const templateSelect = document.getElementById('model-template');
    
    if (!providerId) {
        templateSelect.innerHTML = '<option value="">Please select a provider first</option>';
        return;
    }

    try {
        templateSelect.innerHTML = '<option value="">Loading...</option>';
        const data = await apiCall(`/api/models/templates/${providerId}`);
        modelTemplates[providerId] = data.templates;
        
        templateSelect.innerHTML = '<option value="">Please select a model...</option>' + 
            data.templates.map((template, index) => 
                `<option value="${index}">${escapeHtml(template.display_name)} (${template.name})</option>`
            ).join('');
    } catch (error) {
        templateSelect.innerHTML = '<option value="">Loading failed</option>';
        console.error('Failed to load model templates:', error);
    }
}

// 填充模型模板
function fillModelTemplate() {
    const providerId = document.getElementById('model-provider').value;
    const templateIndex = document.getElementById('model-template').value;
    
    if (!providerId || templateIndex === '') return;
    
    const template = modelTemplates[providerId][parseInt(templateIndex)];
    if (template) {
        document.getElementById('model-type').value = template.model_type;
        document.getElementById('model-cost-input').value = template.recommended_cost_input;
        document.getElementById('model-cost-output').value = template.recommended_cost_output;
    }
}

// 切换自定义模型
function toggleCustomModel() {
    const checkbox = document.getElementById('custom-model-checkbox');
    const customGroup = document.getElementById('custom-model-group');
    const templateSelect = document.getElementById('model-template');
    
    if (checkbox.checked) {
        customGroup.style.display = 'block';
        templateSelect.disabled = true;
        templateSelect.required = false;
        document.getElementById('custom-model-name').required = true;
    } else {
        customGroup.style.display = 'none';
        templateSelect.disabled = false;
        templateSelect.required = true;
        document.getElementById('custom-model-name').required = false;
    }
}

// 初始化表单
function initForms() {
    // Add real-time validation
    initRealTimeValidation();
    
    // Provider表单
    document.getElementById('add-provider-form').addEventListener('submit', async function(e) {
        e.preventDefault();
        
        const name = document.getElementById('provider-name').value.trim();
        const displayName = document.getElementById('provider-display-name').value.trim();
        const baseUrl = document.getElementById('provider-base-url').value.trim();
        
        // Client-side validation
        if (!name) {
            showError('提供商名称是必填项');
            return;
        }
        if (!displayName) {
            showError('显示名称是必填项');
            return;
        }
        
        // Validate base URL format if provided
        if (baseUrl && !isValidUrl(baseUrl)) {
            showError('请输入有效的基础URL');
            return;
        }
        
        const formData = {
            name: name,
            display_name: displayName,
            base_url: baseUrl || null,
            api_key: document.getElementById('provider-api-key').value.trim() || null,
            description: document.getElementById('provider-description').value.trim() || null
        };

        try {
            await apiCall('/api/providers', {
                method: 'POST',
                body: JSON.stringify(formData)
            });
            
            closeModal('add-provider-modal');
            this.reset();
            showSuccess('提供商添加成功！');
            loadProviders();
        } catch (error) {
            showError('添加失败：' + error.message);
        }
    });

    // Model表单
    document.getElementById('add-model-form').addEventListener('submit', async function(e) {
        e.preventDefault();
        
        const isCustom = document.getElementById('custom-model-checkbox').checked;
        let modelName;
        
        if (isCustom) {
            modelName = document.getElementById('custom-model-name').value.trim();
        } else {
            const providerId = document.getElementById('model-provider').value;
            const templateIndex = document.getElementById('model-template').value;
            const template = modelTemplates[providerId][parseInt(templateIndex)];
            modelName = template.name;
        }

        const formData = {
            provider_id: document.getElementById('model-provider').value,
            name: modelName,
            model_type: document.getElementById('model-type').value,
            base_url: document.getElementById('model-base-url').value.trim() || null,
            cost_per_token_input: parseFloat(document.getElementById('model-cost-input').value),
            cost_per_token_output: parseFloat(document.getElementById('model-cost-output').value),
            auto_start: document.getElementById('model-auto-start').checked,
            custom_model: isCustom
        };

        try {
            await apiCall('/api/models', {
                method: 'POST',
                body: JSON.stringify(formData)
            });
            
            closeModal('add-model-modal');
            this.reset();
            showSuccess('模型添加成功！');
            loadModels();
        } catch (error) {
            showError('添加失败：' + error.message);
        }
    });

    // Edit Provider form
    document.getElementById('edit-provider-form').addEventListener('submit', async function(e) {
        e.preventDefault();
        
        const providerId = document.getElementById('edit-provider-id').value;
        const displayName = document.getElementById('edit-provider-display-name').value.trim();
        const baseUrl = document.getElementById('edit-provider-base-url').value.trim();
        
        // Client-side validation
        if (!displayName) {
            showError('显示名称是必填项');
            return;
        }
        
        // Validate base URL format if provided
        if (baseUrl && !isValidUrl(baseUrl)) {
            showError('请输入有效的基础URL');
            return;
        }
        
        const apiKey = document.getElementById('edit-provider-api-key').value.trim();
        
        const formData = {
            display_name: displayName,
            base_url: baseUrl || null,
            api_key: apiKey || null,
            description: document.getElementById('edit-provider-description').value.trim() || null,
            is_active: document.getElementById('edit-provider-active').checked
        };

        try {
            await apiCall(`/api/providers/${providerId}`, {
                method: 'PUT',
                body: JSON.stringify(formData)
            });
            
            closeModal('edit-provider-modal');
            showSuccess('提供商更新成功！');
            loadProviders();
        } catch (error) {
            showError('更新提供商失败：' + error.message);
        }
    });

    // Add API Key form
    document.getElementById('add-api-key-form').addEventListener('submit', async function(e) {
        e.preventDefault();
        
        const providerId = document.getElementById('api-keys-provider-id').value;
        const apiKey = document.getElementById('new-api-key').value.trim();
        
        if (!apiKey) {
            showError('API Key is required');
            return;
        }
        
        const formData = {
            provider_id: providerId,
            api_key: apiKey,
            rate_limit_per_minute: parseInt(document.getElementById('new-api-key-rate-min').value) || null,
            rate_limit_per_hour: parseInt(document.getElementById('new-api-key-rate-hour').value) || null
        };

        try {
            await apiCall('/api/providers/' + providerId + '/api-keys', {
                method: 'POST',
                body: JSON.stringify(formData)
            });
            
            this.reset();
            showSuccess('API key added successfully!');
            await loadApiKeys(providerId);
        } catch (error) {
            showError('Failed to add API key: ' + error.message);
        }
    });

    // Edit API Key form
    document.getElementById('edit-api-key-form').addEventListener('submit', async function(e) {
        e.preventDefault();
        
        const keyId = document.getElementById('edit-api-key-id').value;
        
        const formData = {
            is_active: document.getElementById('edit-api-key-active').checked,
            rate_limit_per_minute: parseInt(document.getElementById('edit-api-key-rate-min').value) || null,
            rate_limit_per_hour: parseInt(document.getElementById('edit-api-key-rate-hour').value) || null
        };

        try {
            await apiCall(`/api/api-keys/${keyId}`, {
                method: 'PUT',
                body: JSON.stringify(formData)
            });
            
            closeModal('edit-api-key-modal');
            showSuccess('API key updated successfully!');
            
            // 重新加载API keys列表
            const providerId = document.getElementById('api-keys-provider-id').value;
            await loadApiKeys(providerId);
        } catch (error) {
            showError('Failed to update API key: ' + error.message);
        }
    });

    // Edit Model form
    document.getElementById('edit-model-form').addEventListener('submit', async function(e) {
        e.preventDefault();
        
        const modelId = document.getElementById('edit-model-id').value;
        const baseUrl = document.getElementById('edit-model-base-url').value.trim();
        const costInput = document.getElementById('edit-model-cost-input').value;
        const costOutput = document.getElementById('edit-model-cost-output').value;
        const config = document.getElementById('edit-model-config').value.trim();
        
        // 验证Base URL格式（如果提供）
        if (baseUrl && !isValidUrl(baseUrl)) {
            showError('Please enter a valid URL for Base URL');
            return;
        }
        
        // 验证JSON配置格式（如果提供）
        if (config) {
            try {
                JSON.parse(config);
            } catch (e) {
                showError('Configuration must be valid JSON');
                return;
            }
        }
        
        const formData = {
            base_url: baseUrl || null,
            is_active: document.getElementById('edit-model-active').checked,
            cost_per_token_input: costInput ? parseFloat(costInput) : null,
            cost_per_token_output: costOutput ? parseFloat(costOutput) : null,
            config: config || null
        };

        try {
            await apiCall(`/api/models/${modelId}`, {
                method: 'PUT',
                body: JSON.stringify(formData)
            });
            
            closeModal('edit-model-modal');
            showSuccess('Model updated successfully!');
            loadModels();
        } catch (error) {
            showError('Failed to update model: ' + error.message);
        }
    });

    // Initialize real-time validation
    initRealTimeValidation();
}

// 关闭模态框
function closeModal(modalId) {
    document.getElementById(modalId).classList.remove('active');
}

// 工具函数
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function isValidUrl(string) {
    try {
        new URL(string);
        return true;
    } catch (_) {
        return false;
    }
}

// Real-time form validation
function initRealTimeValidation() {
    // Add provider form validation
    const providerNameInput = document.getElementById('provider-name');
    const providerDisplayNameInput = document.getElementById('provider-display-name');
    const providerBaseUrlInput = document.getElementById('provider-base-url');
    
    // Edit provider form validation
    const editProviderDisplayNameInput = document.getElementById('edit-provider-display-name');
    const editProviderBaseUrlInput = document.getElementById('edit-provider-base-url');
    
    // Validate required fields
    [providerNameInput, providerDisplayNameInput, editProviderDisplayNameInput].forEach(input => {
        if (input) {
            input.addEventListener('input', function() {
                validateRequiredField(this);
            });
        }
    });
    
    // Validate URL fields
    [providerBaseUrlInput, editProviderBaseUrlInput].forEach(input => {
        if (input) {
            input.addEventListener('input', function() {
                validateUrlField(this);
            });
        }
    });
}

function validateRequiredField(input) {
    const value = input.value.trim();
    if (!value) {
        input.style.borderColor = '#dc3545';
    } else {
        input.style.borderColor = '#28a745';
    }
}

function validateUrlField(input) {
    const value = input.value.trim();
    if (value && !isValidUrl(value)) {
        input.style.borderColor = '#dc3545';
    } else {
        input.style.borderColor = value ? '#28a745' : '#ced4da';
    }
}

function showSuccess(message) {
    showNotification(message, 'success');
}

function showError(message) {
    showNotification(message, 'error');
}

function showNotification(message, type) {
    // Remove any existing notifications
    const existingNotifications = document.querySelectorAll('.notification');
    existingNotifications.forEach(n => n.remove());
    
    const notification = document.createElement('div');
    notification.className = `notification ${type}`;
    notification.innerHTML = `
        <div style="display: flex; align-items: center; justify-content: space-between;">
            <span>${escapeHtml(message)}</span>
            <button onclick="this.parentElement.parentElement.remove()" style="
                background: none; 
                border: none; 
                color: inherit; 
                font-size: 1.2em; 
                cursor: pointer;
                padding: 0;
                margin-left: 1rem;
            ">&times;</button>
        </div>
    `;
    
    // Enhanced styles
    const baseStyles = `
        position: fixed;
        top: 20px;
        right: 20px;
        z-index: 10000;
        max-width: 400px;
        padding: 1rem 1.5rem;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.15);
        font-weight: 500;
        animation: slideIn 0.3s ease-out;
    `;
    
    if (type === 'success') {
        notification.style.cssText = baseStyles + `
            background: #d4edda;
            color: #155724;
            border: 1px solid #c3e6cb;
        `;
    } else {
        notification.style.cssText = baseStyles + `
            background: #f8d7da;
            color: #721c24;
            border: 1px solid #f5c6cb;
        `;
    }
    
    document.body.appendChild(notification);
    
    // Auto remove after delay
    setTimeout(() => {
        if (notification.parentNode) {
            notification.style.animation = 'slideOut 0.3s ease-in';
            setTimeout(() => notification.remove(), 300);
        }
    }, type === 'success' ? 3000 : 5000);
}

// Edit Provider function
async function editProvider(id) {
    try {
        // Fetch the provider details
        const provider = await apiCall(`/api/providers/${id}`);
        
        // Populate the edit form
        document.getElementById('edit-provider-id').value = provider.id;
        document.getElementById('edit-provider-name').value = provider.name;
        document.getElementById('edit-provider-display-name').value = provider.display_name;
        document.getElementById('edit-provider-base-url').value = provider.base_url || '';
        document.getElementById('edit-provider-api-key').value = ''; // Always start empty for new key
        document.getElementById('edit-provider-description').value = provider.description || '';
        document.getElementById('edit-provider-active').checked = provider.is_active;
        
        // Show the edit modal
        document.getElementById('edit-provider-modal').classList.add('active');
    } catch (error) {
        showError('Failed to load provider details: ' + error.message);
    }
}

async function deleteProvider(id) {
    if (confirm('Are you sure you want to delete this Provider? This operation cannot be undone.')) {
        try {
            const response = await apiCall(`/api/providers/${id}`, {
                method: 'DELETE'
            });
            
            // Check if response contains an error (e.g., has associated models)
            if (response.error) {
                showError(response.message || response.error);
            } else {
                showSuccess('Provider deleted successfully!');
                loadProviders(); // Reload the provider list
            }
        } catch (error) {
            showError('Failed to delete provider: ' + error.message);
        }
    }
}

async function editModel(id) {
    try {
        // 获取模型详情
        const model = await apiCall(`/api/models/${id}`);
        
        // 填充编辑表单
        document.getElementById('edit-model-id').value = model.id;
        document.getElementById('edit-model-name').value = model.name;
        document.getElementById('edit-model-provider').value = model.provider_name;
        document.getElementById('edit-model-type').value = model.model_type.toUpperCase();
        document.getElementById('edit-model-base-url').value = model.base_url || '';
        document.getElementById('edit-model-cost-input').value = model.cost_per_token_input || '';
        document.getElementById('edit-model-cost-output').value = model.cost_per_token_output || '';
        document.getElementById('edit-model-config').value = model.config || '';
        document.getElementById('edit-model-active').checked = model.is_active;
        
        // 显示编辑模态框
        document.getElementById('edit-model-modal').classList.add('active');
    } catch (error) {
        showError('Failed to load model details: ' + error.message);
    }
}

async function deleteModel(id) {
    if (confirm('Are you sure you want to delete this Model? This operation cannot be undone.')) {
        try {
            await apiCall(`/api/models/${id}`, {
                method: 'DELETE'
            });
            
            showSuccess('Model deleted successfully!');
            loadModels(); // Reload the model list
        } catch (error) {
            showError('Failed to delete model: ' + error.message);
        }
    }
}

// API Key管理功能
async function manageApiKeys(providerId, providerName) {
    document.getElementById('api-keys-provider-id').value = providerId;
    document.getElementById('api-keys-provider-name').textContent = providerName;
    document.getElementById('api-keys-modal').classList.add('active');
    
    // 加载API Keys
    await loadApiKeys(providerId);
}

async function loadApiKeys(providerId) {
    const loadingEl = document.getElementById('api-keys-loading');
    const errorEl = document.getElementById('api-keys-error');
    const contentEl = document.getElementById('api-keys-content');
    const listEl = document.getElementById('api-keys-list');

    try {
        loadingEl.style.display = 'block';
        errorEl.style.display = 'none';
        contentEl.style.display = 'none';

        const data = await apiCall(`/api/providers/${providerId}/api-keys`);
        
        if (data.keys.length === 0) {
            listEl.innerHTML = '<div style="text-align: center; color: #666; padding: 2rem;">No API keys found for this provider.</div>';
        } else {
            listEl.innerHTML = data.keys.map(key => `
                <div class="api-key-item" style="
                    border: 1px solid #ddd; 
                    border-radius: 8px; 
                    padding: 1rem; 
                    margin-bottom: 1rem; 
                    background: ${key.is_active ? '#f8fff8' : '#fff8f8'};
                ">
                    <div style="display: flex; justify-content: space-between; align-items: center;">
                        <div>
                            <div style="font-weight: 600; margin-bottom: 0.5rem;">
                                ${escapeHtml(key.key_preview)}
                                <span class="status-badge ${key.is_active ? 'status-active' : 'status-inactive'}" style="margin-left: 0.5rem; font-size: 0.8rem;">
                                    ${key.is_active ? 'Active' : 'Inactive'}
                                </span>
                            </div>
                            <div style="font-size: 0.9rem; color: #666;">
                                Usage: ${key.usage_count} calls
                                ${key.last_used_at ? ` • Last used: ${new Date(key.last_used_at).toLocaleString()}` : ' • Never used'}
                            </div>
                            ${key.rate_limit_per_minute || key.rate_limit_per_hour ? `
                                <div style="font-size: 0.9rem; color: #666; margin-top: 0.25rem;">
                                    Rate limits: ${key.rate_limit_per_minute ? key.rate_limit_per_minute + '/min' : ''} 
                                    ${key.rate_limit_per_hour ? key.rate_limit_per_hour + '/hour' : ''}
                                </div>
                            ` : ''}
                        </div>
                        <div style="display: flex; gap: 0.5rem;">
                            <button class="btn btn-secondary" onclick="editApiKey('${key.id}')" style="padding: 0.25rem 0.5rem; font-size: 0.8rem;">
                                Edit
                            </button>
                            <button class="btn ${key.is_active ? 'btn-warning' : 'btn-success'}" 
                                    onclick="toggleApiKeyStatus('${key.id}', ${!key.is_active})" 
                                    style="padding: 0.25rem 0.5rem; font-size: 0.8rem;">
                                ${key.is_active ? 'Disable' : 'Enable'}
                            </button>
                            <button class="btn btn-danger" onclick="deleteApiKey('${key.id}')" style="padding: 0.25rem 0.5rem; font-size: 0.8rem;">
                                Delete
                            </button>
                        </div>
                    </div>
                </div>
            `).join('');
        }

        loadingEl.style.display = 'none';
        contentEl.style.display = 'block';
    } catch (error) {
        loadingEl.style.display = 'none';
        errorEl.textContent = 'Failed to load API keys: ' + error.message;
        errorEl.style.display = 'block';
    }
}

async function editApiKey(keyId) {
    try {
        // 由于安全原因，我们不能直接获取API key的详细信息
        // 这里需要从当前列表中找到对应的key信息
        const providerId = document.getElementById('api-keys-provider-id').value;
        const data = await apiCall(`/api/providers/${providerId}/api-keys`);
        const key = data.keys.find(k => k.id === keyId);
        
        if (!key) {
            showError('API key not found');
            return;
        }

        document.getElementById('edit-api-key-id').value = key.id;
        document.getElementById('edit-api-key-preview').value = key.key_preview;
        document.getElementById('edit-api-key-rate-min').value = key.rate_limit_per_minute || '';
        document.getElementById('edit-api-key-rate-hour').value = key.rate_limit_per_hour || '';
        document.getElementById('edit-api-key-active').checked = key.is_active;
        
        document.getElementById('edit-api-key-modal').classList.add('active');
    } catch (error) {
        showError('Failed to load API key details: ' + error.message);
    }
}

async function toggleApiKeyStatus(keyId, newStatus) {
    try {
        await apiCall(`/api/api-keys/${keyId}/toggle/${newStatus}`, {
            method: 'PUT'
        });
        
        showSuccess(`API key ${newStatus ? 'enabled' : 'disabled'} successfully!`);
        
        // 重新加载API keys列表
        const providerId = document.getElementById('api-keys-provider-id').value;
        await loadApiKeys(providerId);
    } catch (error) {
        showError('Failed to toggle API key status: ' + error.message);
    }
}

async function deleteApiKey(keyId) {
    if (confirm('Are you sure you want to delete this API key? This operation cannot be undone.')) {
        try {
            await apiCall(`/api/api-keys/${keyId}`, {
                method: 'DELETE'
            });
            
            showSuccess('API key deleted successfully!');
            
            // 重新加载API keys列表
            const providerId = document.getElementById('api-keys-provider-id').value;
            await loadApiKeys(providerId);
        } catch (error) {
            showError('Failed to delete API key: ' + error.message);
        }
    }
}

// 模态框点击外部关闭
document.addEventListener('click', function(e) {
    if (e.target.classList.contains('modal')) {
        e.target.classList.remove('active');
    }
});

// ==================== Call Logs Functions ====================

// 加载调用日志统计信息
async function loadCallLogStats() {
    console.log('loadCallLogStats called');
    try {
        const stats = await apiCall('/api/call-logs/stats');
        console.log('Stats loaded:', stats);
        callLogStats = stats.stats;
        displayCallLogStats();
    } catch (error) {
        showError('Failed to load call log statistics: ' + error.message);
        document.getElementById('call-stats-loading').style.display = 'none';
        document.getElementById('call-stats-content').innerHTML = '<div class="error">Failed to load statistics</div>';
        document.getElementById('call-stats-content').style.display = 'block';
    }
}

// 显示调用日志统计信息
function displayCallLogStats() {
    const statsContainer = document.getElementById('call-stats-content');
    const loading = document.getElementById('call-stats-loading');
    
    if (!callLogStats) {
        statsContainer.innerHTML = '<div class="error">No statistics available</div>';
        loading.style.display = 'none';
        statsContainer.style.display = 'block';
        return;
    }

    statsContainer.innerHTML = `
        <div class="stat-card">
            <div class="stat-icon blue">📊</div>
            <div class="stat-info">
                <h3>${callLogStats.total_calls || 0}</h3>
                <p>Total Calls</p>
            </div>
        </div>
        <div class="stat-card">
            <div class="stat-icon green">⚡</div>
            <div class="stat-info">
                <h3>${callLogStats.avg_latency_ms ? Math.round(callLogStats.avg_latency_ms) + 'ms' : 'N/A'}</h3>
                <p>Avg Latency</p>
            </div>
        </div>
        <div class="stat-card">
            <div class="stat-icon orange">🔢</div>
            <div class="stat-info">
                <h3>${callLogStats.total_tokens_output || 0}</h3>
                <p>Total Tokens</p>
            </div>
        </div>
        <div class="stat-card">
            <div class="stat-icon purple">❌</div>
            <div class="stat-info">
                <h3>${callLogStats.error_count || 0}</h3>
                <p>Errors</p>
            </div>
        </div>
    `;
    
    loading.style.display = 'none';
    statsContainer.style.display = 'grid';
}

// 加载调用日志
async function loadCallLogs() {
    console.log('loadCallLogs called');
    try {
        showCallLogsLoading();
        
        const params = new URLSearchParams({
            page: currentCallLogPage,
            limit: callLogPageSize,
            error_only: errorOnlyFilter
        });
        console.log('API params:', params.toString());
        
        const response = await apiCall(`/api/call-logs?${params}`);
        callLogs = response.data || [];
        totalCallLogPages = response.total_pages || 1;
        
        displayCallLogs(response);
    } catch (error) {
        showCallLogsError('Failed to load call logs: ' + error.message);
    }
}

// 显示加载状态
function showCallLogsLoading() {
    document.getElementById('call-logs-loading').style.display = 'block';
    document.getElementById('call-logs-error').style.display = 'none';
    document.getElementById('call-logs-content').style.display = 'none';
}

// 显示错误状态
function showCallLogsError(message) {
    document.getElementById('call-logs-loading').style.display = 'none';
    document.getElementById('call-logs-error').textContent = message;
    document.getElementById('call-logs-error').style.display = 'block';
    document.getElementById('call-logs-content').style.display = 'none';
}

// 显示调用日志
function displayCallLogs(response) {
    const tableBody = document.getElementById('call-logs-table-body');
    const loading = document.getElementById('call-logs-loading');
    const content = document.getElementById('call-logs-content');
    
    if (!callLogs || callLogs.length === 0) {
        tableBody.innerHTML = `
            <tr>
                <td colspan="7" style="text-align: center; padding: 2rem; color: #666;">
                    No call logs found
                </td>
            </tr>
        `;
    } else {
        tableBody.innerHTML = callLogs.map(log => `
            <tr class="call-logs-table">
                <td class="id-cell" title="${log.id}">${log.id.substring(0, 8)}...</td>
                <td>${log.model_id || 'N/A'}</td>
                <td class="status-cell">
                    <span class="status-badge ${log.status_code === 200 ? 'status-200' : 'status-error'}">
                        ${log.status_code}
                    </span>
                </td>
                <td class="duration-cell">${log.total_duration || 0}</td>
                <td class="tokens-cell">${log.tokens_output || 0}</td>
                <td class="error-cell" title="${log.error_message || ''}">${log.error_message || '-'}</td>
                <td>${formatDateTime(log.created_at)}</td>
            </tr>
        `).join('');
    }
    
    // 更新分页信息
    updatePaginationInfo(response);
    
    loading.style.display = 'none';
    content.style.display = 'block';
}

// 更新分页信息
function updatePaginationInfo(response) {
    const { total, page, limit, total_pages } = response;
    const start = (page - 1) * limit + 1;
    const end = Math.min(page * limit, total);
    
    document.getElementById('pagination-info-text').textContent = 
        `Showing ${start}-${end} of ${total} entries`;
    
    document.getElementById('current-page').textContent = page;
    document.getElementById('total-pages').textContent = total_pages;
    
    // 更新分页按钮状态
    const prevBtn = document.getElementById('prev-page-btn');
    const nextBtn = document.getElementById('next-page-btn');
    
    prevBtn.disabled = page <= 1;
    nextBtn.disabled = page >= total_pages;
}

// 切换页面
function changePage(delta) {
    const newPage = currentCallLogPage + delta;
    if (newPage >= 1 && newPage <= totalCallLogPages) {
        currentCallLogPage = newPage;
        loadCallLogs();
    }
}

// 刷新调用日志
function refreshCallLogs() {
    currentCallLogPage = 1;
    loadCallLogs();
    loadCallLogStats();
}

// 格式化日期时间
function formatDateTime(dateStr) {
    if (!dateStr) return 'N/A';
    
    try {
        const date = new Date(dateStr);
        return date.toLocaleString('zh-CN', {
            year: 'numeric',
            month: '2-digit',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit'
        });
    } catch (error) {
        return dateStr;
    }
}

// 初始化Call Logs页面的控件
function initCallLogsControls() {
    // 错误过滤器
    const errorOnlyCheckbox = document.getElementById('errorOnlyFilter');
    if (errorOnlyCheckbox) {
        errorOnlyCheckbox.addEventListener('change', function() {
            errorOnlyFilter = this.checked;
            currentCallLogPage = 1;
            if (currentPage === 'call-logs') {
                loadCallLogs();
            }
        });
    }
    
    // 页面大小选择器
    const pageSizeSelect = document.getElementById('pageSizeSelect');
    if (pageSizeSelect) {
        pageSizeSelect.addEventListener('change', function() {
            callLogPageSize = parseInt(this.value);
            currentCallLogPage = 1;
            if (currentPage === 'call-logs') {
                loadCallLogs();
            }
        });
    }
    
    // 搜索功能（预留）
    const callLogSearch = document.getElementById('callLogSearch');
    if (callLogSearch) {
        callLogSearch.addEventListener('input', function() {
            // TODO: 实现搜索功能
            console.log('Call log search:', this.value);
        });
    }
}
