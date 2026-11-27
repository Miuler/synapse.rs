import { invokeTauri } from '@shared/api';

/** Un dato que el proveedor pide para conectar, además del nombre y el token. */
export interface ConnectionField {
  id: 'server' | 'user';
  required: boolean;
  example: string;
}

export interface ConnectionProvider {
  id: string;
  name: string;
  connection_fields: ConnectionField[];
}

/** Una conexión sin su token: solo la referencia `op://` o si hay uno en el llavero. */
export interface Connection {
  id: string;
  provider: string;
  name: string;
  server: string;
  user: string | null;
  reference: string | null;
  stored: boolean;
}

export interface ConnectionInput {
  /** `null`, una conexión nueva. */
  id: string | null;
  provider: string;
  name: string;
  server: string;
  user: string | null;
  /** `null` la deja como está; vacía, la quita; una referencia `op://` o un token. */
  credential: string | null;
  /** Copiar el token único anterior, si vale para este servidor. */
  use_global: boolean;
}

export const connectionsApi = {
  providers: () => invokeTauri<ConnectionProvider[]>('issues_providers'),
  list: () => invokeTauri<Connection[]>('issues_connections'),
  save: (connection: ConnectionInput) => invokeTauri<Connection>('issues_save_connection', { connection }),
  remove: (id: string) => invokeTauri<void>('issues_delete_connection', { id }),
  /** Con qué cuenta entra su token; `name` es `null` si el servidor no lo dice. */
  check: ({ provider, id }: Connection) =>
    invokeTauri<{ name: string | null }>('issues_check_connection', { provider, connection: id }),
};
