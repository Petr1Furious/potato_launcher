import {useState} from 'react';
import {Lock, LogIn} from 'lucide-react';
import {TokenRequest} from '../types/auth';

interface LoginFormProps {
    onLogin: (tokenRequest: TokenRequest) => Promise<void>;
    loading: boolean;
    error: string | null;
}

export default function LoginForm({onLogin, loading, error}: LoginFormProps) {
    const [token, setToken] = useState('');
    const [tokenError, setTokenError] = useState('');

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();

        if (!token.trim()) {
            setTokenError('Access token is required');
            return;
        }

        setTokenError('');
        await onLogin({token: token.trim()});
    };

    const handleTokenChange = (value: string) => {
        setToken(value);
        if (tokenError) {
            setTokenError('');
        }
    };

    return (
        <div className="min-h-screen bg-gray-900 flex items-center justify-center p-4">
            <div className="max-w-md w-full">
                <div className="bg-gray-800 rounded-lg border border-gray-700 p-8">
                    <div className="text-center mb-8">
                        <div
                            className="mx-auto w-16 h-16 bg-green-500/20 rounded-full flex items-center justify-center mb-4">
                            <Lock className="w-8 h-8 text-green-400"/>
                        </div>
                        <h1 className="text-2xl font-bold text-white mb-2">
                            Modpack Manager
                        </h1>
                    </div>

                    {error && (
                        <div className="mb-6 p-4 bg-red-900/20 border border-red-500 rounded-md">
                            <p className="text-red-400 text-sm">{error}</p>
                        </div>
                    )}

                    <form onSubmit={handleSubmit} className="space-y-6">
                        <div>
                            <label htmlFor="token" className="block text-sm font-medium text-gray-300 mb-2">
                                Access Token *
                            </label>
                            <input
                                id="token"
                                type="password"
                                value={token}
                                onChange={(e) => handleTokenChange(e.target.value)}
                                disabled={loading}
                                className={`w-full px-4 py-3 bg-gray-700 border rounded-md text-white placeholder-gray-400 focus:outline-none focus:ring-2 transition-all duration-200 ${
                                    tokenError
                                        ? 'border-red-500 focus:ring-red-500'
                                        : 'border-gray-600 focus:ring-green-500 focus:border-green-500'
                                } ${loading ? 'opacity-50 cursor-not-allowed' : ''}`}
                                placeholder="Enter your access token..."
                            />
                            {tokenError && <p className="mt-1 text-sm text-red-400">{tokenError}</p>}
                        </div>

                        <button
                            type="submit"
                            disabled={loading || !token.trim()}
                            className={`w-full flex items-center justify-center gap-2 py-3 px-6 rounded-md font-medium transition-all duration-200 ${
                                loading || !token.trim()
                                    ? 'bg-gray-600 text-gray-400 cursor-not-allowed'
                                    : 'bg-green-500 hover:bg-green-600 text-white focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 focus:ring-offset-gray-800'
                            }`}
                        >
                            {loading ? (
                                <>
                                    <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                                    Signing in...
                                </>
                            ) : (
                                <>
                                    <LogIn size={20}/>
                                    Sign In
                                </>
                            )}
                        </button>
                    </form>


                </div>
            </div>
        </div>
    );
}