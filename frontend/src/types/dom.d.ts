import 'react';

declare module 'react' {
    interface InputHTMLAttributes<T> {
        webkitdirectory?: string;
        directory?: boolean;
        mozdirectory?: boolean;
    }
}