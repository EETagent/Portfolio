import type { AxiosError } from 'axios';

export type Fetch = (input: RequestInfo, init?: RequestInit) => Promise<Response>;

export const API_URL = 'http://localhost:8000';

export interface ApiError {
	error: AxiosError | unknown;
	msg: string;
}

export const errorHandler = (error: AxiosError | unknown, msg: string): ApiError => {
	return { error, msg };
}
