import { faGlobe } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Extension, ExtensionContext } from 'shared';
import { z } from 'zod';
import { type FieldDef, insertFieldsAfter } from '@/elements/form-engine/index.ts';
import AdminConfigurationPage from './pages/admin/AdminConfigurationPage.tsx';
import ServerSubdomainsPage from './pages/server/ServerSubdomainsPage.tsx';
import { getExtTranslations } from './translations.ts';

class CaloptreyxSubdomainsExtension extends Extension {
  public cardConfigurationPage: React.FC | null = AdminConfigurationPage;
  public cardIcon: React.ReactNode = <FontAwesomeIcon icon={faGlobe} />;

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.enterRoutes((routes) =>
      routes.addServerRoute({
        name: () => getExtTranslations().t('pages.server.subdomains.title', {}),
        icon: faGlobe,
        path: '/subdomains',
        element: ServerSubdomainsPage,
        permission: 'subdomains.read',
      }),
    );

    ctx.extensionRegistry.enterPermissionIcons((icons) =>
      icons
        .addServerPermissionIcon('subdomains', <FontAwesomeIcon icon={faGlobe} />)
        .addAdminPermissionIcon('subdomains', <FontAwesomeIcon icon={faGlobe} />),
    );

    ctx.extensionRegistry.enterForms((forms) => {
      for (const formId of ['admin.servers.create', 'admin.servers.update'] as const) {
        forms.extend(formId, {
          zodShape: {
            featureLimits: z.object({
              subdomains: z.number().int().min(0),
            }),
          },
          initialValues: {
            featureLimits: {
              subdomains: 0,
            },
          },
          transform: (fields) =>
            insertFieldsAfter(fields, 'featureLimits.schedules', {
              type: 'number',
              name: 'featureLimits.subdomains',
              label: () => getExtTranslations().t('serverForm.subdomainsLimit', {}),
              description: () => getExtTranslations().t('serverForm.subdomainsLimitDescription', {}),
              required: true,
              props: { placeholder: '0', min: 0 },
            } satisfies FieldDef),
        });
      }
    });
  }
}

export default new CaloptreyxSubdomainsExtension();
