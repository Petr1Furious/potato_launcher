import {useState, useEffect, useCallback} from 'react';
import * as React from "react";
import {Upload} from 'lucide-react';
import {apiService} from '../services/api';
import {ModpackBase, LoaderType, AuthKind, AuthConfig} from '../types/api';

interface ModpackFormProps {
    onSubmit: (data: ModpackBase) => void;
}

export default function ModpackForm({onSubmit}: ModpackFormProps) {
    const [formData, setFormData] = useState<ModpackBase>({
        name: '',
        minecraft_version: '',
        loader: LoaderType.VANILLA,
        loader_version: '',
        auth_config: {
            kind: AuthKind.OFFLINE
        }
    });

    const [minecraftVersions, setMinecraftVersions] = useState<string[]>([]);
    const [availableLoaders, setAvailableLoaders] = useState<string[]>([]);
    const [loaderVersions, setLoaderVersions] = useState<string[]>([]);
    const [loading, setLoading] = useState(false);
    const [errors, setErrors] = useState<{ [key: string]: string }>({});
    const [dragActive, setDragActive] = useState(false);
    const [uploadedFiles, setUploadedFiles] = useState<FileList | null>(null);

    useEffect(() => {
        loadMinecraftVersions();
    }, []);

    const loadMinecraftVersions = async () => {
        try {
            const versions = await apiService.getMinecraftVersions();
            setMinecraftVersions(versions);
        } catch (err) {
            console.error('Failed to load Minecraft versions:', err);
        }
    };

    const loadLoaders = useCallback(async (version: string) => {
        if (!version) {
            setAvailableLoaders([]);
            return;
        }

        try {
            const loaders = await apiService.getLoadersForVersion(version);
            setAvailableLoaders(loaders);
        } catch (err) {
            console.error('Failed to load loaders:', err);
            setAvailableLoaders([]);
        }
    }, []);

    const loadLoaderVersions = useCallback(async (mcVersion: string, loader: string) => {
        if (!mcVersion || !loader) {
            setLoaderVersions([]);
            return;
        }

        try {
            const versions = await apiService.getLoaderVersions(mcVersion, loader);
            setLoaderVersions(versions);
        } catch (err) {
            console.error('Failed to load loader versions:', err);
            setLoaderVersions([]);
        }
    }, []);

    useEffect(() => {
        if (formData.minecraft_version) {
            loadLoaders(formData.minecraft_version);
            setFormData(prev => ({...prev, loader: LoaderType.VANILLA, loader_version: ''}));
        }
    }, [formData.minecraft_version, loadLoaders]);

    useEffect(() => {
        if (formData.minecraft_version && formData.loader) {
            loadLoaderVersions(formData.minecraft_version, formData.loader);
            setFormData(prev => ({...prev, loader_version: ''}));
        }
    }, [formData.minecraft_version, formData.loader, loadLoaderVersions]);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();

        const newErrors: { [key: string]: string } = {};

        if (!formData.name.trim()) {
            newErrors.name = 'Name is required';
        }
        if (!formData.minecraft_version) {
            newErrors.minecraft_version = 'Minecraft version is required';
        }
        if (!formData.loader) {
            newErrors.loader = 'Loader is required';
        }
        if (!formData.loader_version) {
            newErrors.loader_version = 'Loader version is required';
        }
        if (!formData.auth_config.kind) {
            newErrors.auth_kind = 'Authentication type is required';
        }
        if (formData.auth_config.kind === AuthKind.TELEGRAM && !formData.auth_config.auth_base_url?.trim()) {
            newErrors.auth_base_url = 'Auth base URL is required for Telegram';
        }
        if (formData.auth_config.kind === AuthKind.ELY_BY) {
            if (!formData.auth_config.client_id?.trim()) {
                newErrors.client_id = 'Client ID is required for Ely.by';
            }
            if (!formData.auth_config.client_secret?.trim()) {
                newErrors.client_secret = 'Client Secret is required for Ely.by';
            }
        }

        setErrors(newErrors);

        if (Object.keys(newErrors).length === 0) {
            setLoading(true);
            try {
                // Create modpack first
                const newModpack = await apiService.createModpack(formData);

                // Upload files if selected
                if (uploadedFiles && uploadedFiles.length > 0) {
                    await apiService.uploadModpackFiles(newModpack.id, uploadedFiles);
                }

                // Call parent callback
                onSubmit(formData);

                // Reset form
                setFormData({
                    name: '',
                    minecraft_version: '',
                    loader: LoaderType.VANILLA,
                    loader_version: '',
                    auth_config: {
                        kind: AuthKind.OFFLINE
                    }
                });
                setUploadedFiles(null);
            } catch (err) {
                console.error('Failed to create modpack:', err);
                setErrors({submit: err instanceof Error ? err.message : 'Failed to create modpack'});
            } finally {
                setLoading(false);
            }
        }
    };

    const handleInputChange = (field: keyof ModpackBase, value: string | LoaderType) => {
        setFormData(prev => ({...prev, [field]: value}));
        if (errors[field]) {
            setErrors(prev => ({...prev, [field]: ''}));
        }
    };

    const handleAuthConfigChange = (field: keyof AuthConfig, value: string | AuthKind) => {
        setFormData(prev => ({
            ...prev,
            auth_config: {
                ...prev.auth_config,
                [field]: value,
                // Clear optional fields when changing kind
                ...(field === 'kind' && {
                    auth_base_url: undefined,
                    client_id: undefined,
                    client_secret: undefined
                })
            }
        }));
        if (errors[field as string]) {
            setErrors(prev => ({...prev, [field as string]: ''}));
        }
    };
    const handleDrag = (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        if (e.type === 'dragenter' || e.type === 'dragover') {
            setDragActive(true);
        } else if (e.type === 'dragleave') {
            setDragActive(false);
        }
    };

    const handleDrop = (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setDragActive(false);

        if (e.dataTransfer.files && e.dataTransfer.files[0]) {
            setUploadedFiles(e.dataTransfer.files);
        }
    };

    const handleFileInput = (e: React.ChangeEvent<HTMLInputElement>) => {
        if (e.target.files && e.target.files[0]) {
            setUploadedFiles(e.target.files);
        }
    };

    return (
        <div className="max-w-2xl mx-auto">
            <div className="bg-gray-800 rounded-lg border border-gray-700 p-8">
                <h2 className="text-2xl font-bold text-white mb-6">Create New Modpack</h2>

                <form onSubmit={handleSubmit} className="space-y-6">
                    {errors.submit && (
                        <div className="p-4 bg-red-900/20 border border-red-500 rounded-md">
                            <p className="text-red-400 text-sm">{errors.submit}</p>
                        </div>
                    )}

                    <div>
                        <label htmlFor="name" className="block text-sm font-medium text-gray-300 mb-2">
                            Modpack Name *
                        </label>
                        <input
                            id="name"
                            type="text"
                            value={formData.name}
                            onChange={(e) => handleInputChange('name', e.target.value)}
                            className={`w-full px-4 py-3 bg-gray-700 border rounded-md text-white placeholder-gray-400 focus:outline-none focus:ring-2 transition-all duration-200 ${
                                errors.name
                                    ? 'border-red-500 focus:ring-red-500'
                                    : 'border-gray-600 focus:ring-green-500 focus:border-green-500'
                            }`}
                            placeholder="Enter modpack name..."
                        />
                        {errors.name && <p className="mt-1 text-sm text-red-400">{errors.name}</p>}
                    </div>

                    <div>
                        <label htmlFor="minecraft_version" className="block text-sm font-medium text-gray-300 mb-2">
                            Minecraft Version *
                        </label>
                        <select
                            id="minecraft_version"
                            value={formData.minecraft_version}
                            onChange={(e) => handleInputChange('minecraft_version', e.target.value)}
                            className={`w-full px-4 py-3 bg-gray-700 border rounded-md text-white focus:outline-none focus:ring-2 transition-all duration-200 ${
                                errors.minecraft_version
                                    ? 'border-red-500 focus:ring-red-500'
                                    : 'border-gray-600 focus:ring-green-500 focus:border-green-500'
                            }`}
                        >
                            <option value="">Select Minecraft version...</option>
                            {minecraftVersions.map((version) => (
                                <option key={version} value={version}>
                                    {version}
                                </option>
                            ))}
                        </select>
                        {errors.minecraft_version &&
                            <p className="mt-1 text-sm text-red-400">{errors.minecraft_version}</p>}
                    </div>

                    <div>
                        <label htmlFor="loader" className="block text-sm font-medium text-gray-300 mb-2">
                            Mod Loader *
                        </label>
                        <select
                            id="loader"
                            value={formData.loader}
                            onChange={(e) => handleInputChange('loader', e.target.value as LoaderType)}
                            disabled={!formData.minecraft_version || availableLoaders.length === 0}
                            className={`w-full px-4 py-3 bg-gray-700 border rounded-md text-white focus:outline-none focus:ring-2 transition-all duration-200 ${
                                !formData.minecraft_version || availableLoaders.length === 0
                                    ? 'opacity-50 cursor-not-allowed border-gray-600'
                                    : errors.loader
                                        ? 'border-red-500 focus:ring-red-500'
                                        : 'border-gray-600 focus:ring-green-500 focus:border-green-500'
                            }`}
                        >
                            <option value="">Select mod loader...</option>
                            {availableLoaders.map((loader) => (
                                <option key={loader} value={loader}>
                                    {loader.charAt(0).toUpperCase() + loader.slice(1)}
                                </option>
                            ))}
                        </select>
                        {errors.loader && <p className="mt-1 text-sm text-red-400">{errors.loader}</p>}
                        {!formData.minecraft_version && (
                            <p className="mt-1 text-sm text-gray-500">Select a Minecraft version first</p>
                        )}
                        {formData.minecraft_version && availableLoaders.length === 0 && (
                            <p className="mt-1 text-sm text-yellow-400">No loaders available for this version</p>
                        )}
                    </div>

                    <div>
                        <label htmlFor="loader_version" className="block text-sm font-medium text-gray-300 mb-2">
                            Loader Version *
                        </label>
                        <select
                            id="loader_version"
                            value={formData.loader_version}
                            onChange={(e) => handleInputChange('loader_version', e.target.value)}
                            disabled={!formData.loader || loaderVersions.length === 0}
                            className={`w-full px-4 py-3 bg-gray-700 border rounded-md text-white focus:outline-none focus:ring-2 transition-all duration-200 ${
                                !formData.loader || loaderVersions.length === 0
                                    ? 'opacity-50 cursor-not-allowed border-gray-600'
                                    : errors.loader_version
                                        ? 'border-red-500 focus:ring-red-500'
                                        : 'border-gray-600 focus:ring-green-500 focus:border-green-500'
                            }`}
                        >
                            <option value="">Select loader version...</option>
                            {loaderVersions.map((version) => (
                                <option key={version} value={version}>
                                    {version}
                                </option>
                            ))}
                        </select>
                        {errors.loader_version && <p className="mt-1 text-sm text-red-400">{errors.loader_version}</p>}
                        {!formData.loader && (
                            <p className="mt-1 text-sm text-gray-500">Select a mod loader first</p>
                        )}
                        {formData.loader && loaderVersions.length === 0 && (
                            <p className="mt-1 text-sm text-yellow-400">No versions available for this loader</p>
                        )}
                    </div>

                    <div>
                        <label htmlFor="auth_kind" className="block text-sm font-medium text-gray-300 mb-2">
                            Authentication Type *
                        </label>
                        <select
                            id="auth_kind"
                            value={formData.auth_config.kind}
                            onChange={(e) => handleAuthConfigChange('kind', e.target.value as AuthKind)}
                            className={`w-full px-4 py-3 bg-gray-700 border rounded-md text-white focus:outline-none focus:ring-2 transition-all duration-200 ${
                                errors.auth_kind
                                    ? 'border-red-500 focus:ring-red-500'
                                    : 'border-gray-600 focus:ring-green-500 focus:border-green-500'
                            }`}
                        >
                            <option value={AuthKind.OFFLINE}>Offline</option>
                            <option value={AuthKind.MOJANG}>Mojang</option>
                            <option value={AuthKind.TELEGRAM}>Telegram</option>
                            <option value={AuthKind.ELY_BY}>Ely.by</option>
                        </select>
                        {errors.auth_kind && <p className="mt-1 text-sm text-red-400">{errors.auth_kind}</p>}
                    </div>

                    {formData.auth_config.kind === AuthKind.TELEGRAM && (
                        <div>
                            <label htmlFor="auth_base_url" className="block text-sm font-medium text-gray-300 mb-2">
                                Auth Base URL *
                            </label>
                            <input
                                id="auth_base_url"
                                type="url"
                                value={formData.auth_config.auth_base_url || ''}
                                onChange={(e) => handleAuthConfigChange('auth_base_url', e.target.value)}
                                className={`w-full px-4 py-3 bg-gray-700 border rounded-md text-white placeholder-gray-400 focus:outline-none focus:ring-2 transition-all duration-200 ${
                                    errors.auth_base_url
                                        ? 'border-red-500 focus:ring-red-500'
                                        : 'border-gray-600 focus:ring-green-500 focus:border-green-500'
                                }`}
                                placeholder="https://your-telegram-auth-server.com"
                            />
                            {errors.auth_base_url && <p className="mt-1 text-sm text-red-400">{errors.auth_base_url}</p>}
                        </div>
                    )}

                    {formData.auth_config.kind === AuthKind.ELY_BY && (
                        <>
                            <div>
                                <label htmlFor="client_id" className="block text-sm font-medium text-gray-300 mb-2">
                                    Client ID *
                                </label>
                                <input
                                    id="client_id"
                                    type="text"
                                    value={formData.auth_config.client_id || ''}
                                    onChange={(e) => handleAuthConfigChange('client_id', e.target.value)}
                                    className={`w-full px-4 py-3 bg-gray-700 border rounded-md text-white placeholder-gray-400 focus:outline-none focus:ring-2 transition-all duration-200 ${
                                        errors.client_id
                                            ? 'border-red-500 focus:ring-red-500'
                                            : 'border-gray-600 focus:ring-green-500 focus:border-green-500'
                                    }`}
                                    placeholder="Enter Ely.by client ID..."
                                />
                                {errors.client_id && <p className="mt-1 text-sm text-red-400">{errors.client_id}</p>}
                            </div>
                            <div>
                                <label htmlFor="client_secret" className="block text-sm font-medium text-gray-300 mb-2">
                                    Client Secret *
                                </label>
                                <input
                                    id="client_secret"
                                    type="password"
                                    value={formData.auth_config.client_secret || ''}
                                    onChange={(e) => handleAuthConfigChange('client_secret', e.target.value)}
                                    className={`w-full px-4 py-3 bg-gray-700 border rounded-md text-white placeholder-gray-400 focus:outline-none focus:ring-2 transition-all duration-200 ${
                                        errors.client_secret
                                            ? 'border-red-500 focus:ring-red-500'
                                            : 'border-gray-600 focus:ring-green-500 focus:border-green-500'
                                    }`}
                                    placeholder="Enter Ely.by client secret..."
                                />
                                {errors.client_secret && <p className="mt-1 text-sm text-red-400">{errors.client_secret}</p>}
                            </div>
                        </>
                    )}

                    <div>
                        <label className="block text-sm font-medium text-gray-300 mb-2">
                            Upload Modpack Files (Optional)
                        </label>
                        <div
                            className={`relative border-2 border-dashed rounded-lg p-8 text-center transition-all duration-200 ${
                                dragActive
                                    ? 'border-green-500 bg-green-500/10'
                                    : 'border-gray-600 hover:border-gray-500'
                            }`}
                            onDragEnter={handleDrag}
                            onDragLeave={handleDrag}
                            onDragOver={handleDrag}
                            onDrop={handleDrop}
                        >
                            <input
                                type="file"
                                multiple
                                onChange={handleFileInput}
                                className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                                webkitdirectory=""
                                disabled={loading}
                            />
                            <Upload className="mx-auto h-12 w-12 text-gray-400 mb-4"/>
                            <p className="text-gray-300 mb-2">
                                Drag and drop modpack folder here, or click to browse
                            </p>
                            <p className="text-sm text-gray-500">
                                Upload your modpack files and folders
                            </p>
                            {uploadedFiles && (
                                <div className="mt-4 p-3 bg-gray-700 rounded-md">
                                    <p className="text-green-400 text-sm">
                                        {uploadedFiles.length} file(s) selected
                                    </p>
                                </div>
                            )}
                        </div>
                    </div>

                    <div className="pt-4">
                        <button
                            type="submit"
                            disabled={loading}
                            className={`w-full font-bold py-3 px-6 rounded-md transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 focus:ring-offset-gray-800 ${
                                loading
                                    ? 'bg-gray-600 text-gray-400 cursor-not-allowed'
                                    : 'bg-green-500 hover:bg-green-600 text-white'
                            }`}
                        >
                            {loading ? (
                                <div className="flex items-center justify-center gap-2">
                                    <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                                    Creating...
                                </div>
                            ) : (
                                'Create Modpack'
                            )}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}