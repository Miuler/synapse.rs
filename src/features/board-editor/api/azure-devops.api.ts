// La credencial de Azure DevOps, hasta que cada conexión tenga la suya.
import { invokeTauri } from '@shared/api';

/** De dónde sale el token, sin su valor: una referencia `op://` o uno guardado en el llavero. */
export interface AzureTokenSetting {
  reference: string | null;
  stored: boolean;
}

/** El token es el mismo para todos los tableros. */
export const azureToken = {
  get: () => invokeTauri<AzureTokenSetting>('get_azure_token_setting'),
  /** Una referencia `op://` o un token; vacío, lo quita. Devuelve cómo quedó. */
  set: (value: string) => invokeTauri<AzureTokenSetting>('set_azure_token', { value }),
};
