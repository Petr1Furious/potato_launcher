import {useState, useEffect, useCallback, useRef} from 'react';
import {Save} from 'lucide-react';
import {apiService} from '../services/api';
import {SettingResponse, SettingType} from '../types/api';


interface SettingsFormProps {
    onSave: (settings: SettingResponse[]) => void;
}

export default function SettingsForm({onSave}: SettingsFormProps) {
    const [_, setSettings] = useState<SettingResponse[]>([]);
    const [originalSettings, setOriginalSettings] = useState<SettingResponse[]>([]);
    const [displaySettings, setDisplaySettings] = useState<SettingResponse[]>([]);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [hasChanges, setHasChanges] = useState(false);
    const fetchingRef = useRef(false);


    // Default settings that should always be displayed
    const defaultSettings: SettingResponse[] = [
        {key: 'download_server_base', value: '', type: SettingType.STRING},
        {key: 'resources_url_base', value: '', type: SettingType.STRING},
        {key: 'replace_download_urls', value: false, type: SettingType.BOOLEAN},
        {key: 'version_manifest_url', value: '', type: SettingType.STRING},
    ];

    useEffect(() => {
        if (fetchingRef.current) return;
        fetchingRef.current = true;

        loadSettings();
    }, []);

    const loadSettings = async () => {
        try {
            setLoading(true);
            setError(null);
            const data = await apiService.getSettings();
            setSettings(data);
            setOriginalSettings(data);

            // Merge server settings with default settings
            const mergedSettings = defaultSettings.map(defaultSetting => {
                const serverSetting = data.find(s => s.key === defaultSetting.key);
                return serverSetting || defaultSetting;
            });
            setDisplaySettings(mergedSettings);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to load settings');
            // If server fails, show default settings
            setDisplaySettings(defaultSettings);
        } finally {
            setLoading(false);
        }
    };

    const checkForChanges = useCallback(() => {
        const changed = displaySettings.some((setting) => {
            const original = originalSettings.find(s => s.key === setting.key);
            const defaultSetting = defaultSettings.find(s => s.key === setting.key);
            const originalValue = original?.value ?? defaultSetting?.value;
            return setting.value !== originalValue;
        });
        setHasChanges(changed);
    }, [displaySettings, originalSettings]);

    useEffect(() => {
        checkForChanges();
    }, [checkForChanges]);

    const handleInputChange = (key: string, value: string | boolean) => {
        setDisplaySettings(prev => prev.map(setting =>
            setting.key === key ? {...setting, value} : setting
        ));
    };

    const handleSave = async () => {
        try {
            setSaving(true);
            setError(null);

            // Only send changed settings to server
            const changedSettings = displaySettings.filter((setting) => {
                const original = originalSettings.find(s => s.key === setting.key);
                const defaultSetting = defaultSettings.find(s => s.key === setting.key);
                const originalValue = original?.value ?? defaultSetting?.value;
                return setting.value !== originalValue;
            });

            await apiService.updateSettings(changedSettings);

            // Update original settings with current display settings
            const newOriginalSettings = [...originalSettings];
            displaySettings.forEach(displaySetting => {
                const existingIndex = newOriginalSettings.findIndex(s => s.key === displaySetting.key);
                if (existingIndex >= 0) {
                    newOriginalSettings[existingIndex] = displaySetting;
                } else {
                    newOriginalSettings.push(displaySetting);
                }
            });
            setOriginalSettings(newOriginalSettings);
            setHasChanges(false);
            onSave(displaySettings);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to save settings');
        } finally {
            setSaving(false);
        }
    };

    const getSetting = (key: string) => {
        return displaySettings.find(s => s.key === key);
    };

    if (loading) {
        return (
            <div className="max-w-2xl mx-auto">
                <div className="bg-gray-800 rounded-lg border border-gray-700 p-8">
                    <div className="text-center text-white">Loading settings...</div>
                </div>
            </div>
        );
    }

    if (error && !displaySettings.length) {
        return (
            <div className="max-w-2xl mx-auto">
                <div className="bg-gray-800 rounded-lg border border-gray-700 p-8">
                    <div className="text-center">
                        <div className="text-red-400 mb-4">Error: {error}</div>
                        <button
                            onClick={loadSettings}
                            className="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded"
                        >
                            Retry
                        </button>
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="max-w-2xl mx-auto">
            <div className="bg-gray-800 rounded-lg border border-gray-700 p-8">
                <h2 className="text-2xl font-bold text-white mb-6">Application Settings</h2>

                {error && (
                    <div className="mb-4 p-3 bg-red-900/20 border border-red-500 rounded-md">
                        <p className="text-red-400 text-sm">{error}</p>
                    </div>
                )}

                <div className="space-y-6">
                    {getSetting('download_server_base') && (
                        <div>
                            <label htmlFor="download_server_base"
                                   className="block text-sm font-medium text-gray-300 mb-2">
                                Download Server Base URL
                            </label>
                            <input
                                id="download_server_base"
                                type="text"
                                value={getSetting('download_server_base')?.value as string || ''}
                                onChange={(e) => handleInputChange('download_server_base', e.target.value)}
                                className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-md text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-green-500 focus:border-green-500 transition-all duration-200"
                                placeholder="https://your-server.com"
                            />
                            <p className="mt-1 text-sm text-gray-500">The base URL where the instance will be
                                deployed</p>
                        </div>
                    )}

                    {getSetting('resources_url_base') && (
                        <div>
                            <label htmlFor="resources_url_base"
                                   className="block text-sm font-medium text-gray-300 mb-2">
                                Resources URL Base
                            </label>
                            <input
                                id="resources_url_base"
                                type="text"
                                value={getSetting('resources_url_base')?.value as string || ''}
                                onChange={(e) => handleInputChange('resources_url_base', e.target.value)}
                                className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-md text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-green-500 focus:border-green-500 transition-all duration-200"
                                placeholder="https://your-server.com/assets/objects"
                            />
                            <p className="mt-1 text-sm text-gray-500">
                                The base URL for assets. Should be equal to &lt;download_server_base&gt;/assets/objects
                                if the generated folder structure is not changed after upload. If omitted, the launcher
                                will download assets from Mojang servers.
                            </p>
                        </div>
                    )}

                    {getSetting('replace_download_urls') && (
                        <div>
                            <label htmlFor="replace_download_urls"
                                   className="block text-sm font-medium text-gray-300 mb-2">
                                Replace Download URLs
                            </label>
                            <select
                                id="replace_download_urls"
                                value={getSetting('replace_download_urls')?.value?.toString() || 'false'}
                                onChange={(e) => handleInputChange('replace_download_urls', e.target.value === 'true')}
                                className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-green-500 focus:border-green-500 transition-all duration-200"
                            >
                                <option value="false">false</option>
                                <option value="true">true</option>
                            </select>
                            <div className="mt-1 text-sm text-gray-500">
                                <p className="mb-2">
                                    <strong>If true:</strong> All instance files will be downloaded from your server.
                                </p>
                                <p>
                                    <strong>If false:</strong> Original download URLs will be kept when possible.
                                    Assets, libraries, modloaders and the Minecraft jar will be downloaded from their
                                    original locations and only metadata, files specified in include, and (Neo)Forge
                                    patched jars will be downloaded from your server.
                                </p>
                            </div>
                        </div>
                    )}

                    {getSetting('version_manifest_url') && (
                        <div>
                            <label htmlFor="version_manifest_url"
                                   className="block text-sm font-medium text-gray-300 mb-2">
                                Version Manifest URL
                            </label>
                            <input
                                id="version_manifest_url"
                                type="text"
                                value={getSetting('version_manifest_url')?.value as string || ''}
                                onChange={(e) => handleInputChange('version_manifest_url', e.target.value)}
                                className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-md text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-green-500 focus:border-green-500 transition-all duration-200"
                                placeholder="https://your-server.com/version_manifest.json"
                            />
                            <div className="mt-1 text-sm text-gray-500">
                                <p className="mb-2">
                                    The URL from which to fetch a remote version manifest. If specified, the instance
                                    builder will fetch the existing manifest from this URL and merge the local versions
                                    with it, preserving any versions that exist in the remote manifest but not in the
                                    local specification.
                                </p>
                                <p>
                                    Set this to &lt;download_server_base&gt;/version_manifest.json if you want to manage
                                    different instances from different devices (for example, when you have multiple
                                    server admins responsible for different servers).
                                </p>
                            </div>
                        </div>
                    )}

                    <div className="pt-4">
                        <button
                            onClick={handleSave}
                            disabled={!hasChanges || saving}
                            className={`flex items-center gap-2 px-6 py-3 rounded-md font-medium transition-all duration-200 ${
                                hasChanges && !saving
                                    ? 'bg-green-500 hover:bg-green-600 text-white cursor-pointer'
                                    : 'bg-gray-600 text-gray-400 cursor-not-allowed'
                            }`}
                        >
                            <Save size={20}/>
                            {saving ? 'Saving...' : 'Save Settings'}
                        </button>
                        {hasChanges && (
                            <p className="mt-2 text-sm text-yellow-400">You have unsaved changes</p>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}