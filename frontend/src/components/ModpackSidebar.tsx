import {Plus, Settings} from 'lucide-react';
import {LogOut, Hammer} from 'lucide-react';
import {ModpackResponse} from '../types/api';

interface ModpackSidebarProps {
    modpacks: ModpackResponse[];
    selectedModpack: number | null;
    onSelectModpack: (id: number) => void;
    onNewModpack: () => void;
    showForm: boolean;
    onShowSettings: () => void;
    showSettings: boolean;
    onLogout: () => void;
    onBuild: () => void;
}

export default function ModpackSidebar(
    {
        modpacks,
        selectedModpack,
        onSelectModpack,
        onNewModpack,
        showForm,
        onShowSettings,
        showSettings,
        onLogout,
        onBuild
    }: ModpackSidebarProps) {
    return (
        <div className="w-80 h-screen sticky top-0 bg-gray-800 border-r border-gray-700 flex flex-col">
            <div className="p-6 border-b border-gray-700">
                <h1 className="text-2xl font-bold text-white mb-4">Modpack Manager</h1>
                <button
                    onClick={onNewModpack}
                    className={`w-full flex items-center gap-2 px-4 py-3 rounded-md text-white font-medium transition-all duration-200 ${
                        showForm
                            ? 'bg-green-600 hover:bg-green-700'
                            : 'bg-green-500 hover:bg-green-600'
                    }`}
                >
                    <Plus size={20}/>
                    New Modpack
                </button>
                <button
                    onClick={onBuild}
                    className="w-full flex items-center gap-2 px-4 py-3 rounded-md text-white font-medium transition-all duration-200 bg-orange-500 hover:bg-orange-600 mt-3"
                >
                    <Hammer size={20}/>
                    Build
                </button>
            </div>

            <div className="flex-1 overflow-y-auto">
                <div className="p-4">
                    <h2 className="text-gray-400 text-sm font-medium uppercase tracking-wide mb-3">
                        Existing Modpacks
                    </h2>
                    {modpacks.length === 0 ? (
                        <p className="text-gray-500 text-sm italic">No modpacks yet</p>
                    ) : (
                        <div className="space-y-2">
                            {modpacks.map((modpack) => (
                                <button
                                    key={modpack.id}
                                    onClick={() => onSelectModpack(modpack.id)}
                                    className={`w-full text-left p-3 rounded-md transition-all duration-200 border ${
                                        selectedModpack === modpack.id
                                            ? 'bg-green-900/30 border-green-500 text-green-400'
                                            : 'bg-gray-700/50 border-gray-600 text-gray-300 hover:bg-gray-700 hover:border-gray-500'
                                    }`}
                                >
                                    <div className="font-medium">{modpack.name}</div>
                                    <div className="text-sm text-gray-400">{modpack.minecraft_version}</div>
                                </button>
                            ))}
                        </div>
                    )}
                </div>
            </div>

            <div className="p-4 border-t border-gray-700 bg-gray-800">
                <div className="space-y-2">
                    <button
                        onClick={onShowSettings}
                        className={`w-full flex items-center gap-2 px-4 py-3 rounded-md text-white font-medium transition-all duration-200 ${
                            showSettings
                                ? 'bg-blue-600 hover:bg-blue-700'
                                : 'bg-gray-700 hover:bg-gray-600'
                        }`}
                    >
                        <Settings size={20}/>
                        Settings
                    </button>
                    <button
                        onClick={onLogout}
                        className="w-full flex items-center gap-2 px-4 py-3 rounded-md text-white font-medium transition-all duration-200 bg-red-600 hover:bg-red-700"
                    >
                        <LogOut size={20}/>
                        Logout
                    </button>
                </div>
            </div>
        </div>
    );
}
