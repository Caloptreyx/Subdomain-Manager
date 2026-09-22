import Tabs from '@/elements/layout/Tabs.tsx';
import { useExtTranslations } from '../../translations.ts';
import DomainsTab from './DomainsTab.tsx';
import SettingsTab from './SettingsTab.tsx';
import SubdomainsTab from './SubdomainsTab.tsx';

export default function AdminConfigurationPage() {
  const { t: tExt } = useExtTranslations();

  return (
    <Tabs defaultValue='domains'>
      <Tabs.List>
        <Tabs.Tab value='domains'>{tExt('pages.admin.subdomains.tabs.domains', {})}</Tabs.Tab>
        <Tabs.Tab value='settings'>{tExt('pages.admin.subdomains.tabs.settings', {})}</Tabs.Tab>
        <Tabs.Tab value='subdomains'>{tExt('pages.admin.subdomains.tabs.subdomains', {})}</Tabs.Tab>
      </Tabs.List>

      <Tabs.Panel value='domains' pt='md'>
        <DomainsTab />
      </Tabs.Panel>
      <Tabs.Panel value='settings' pt='md'>
        <SettingsTab />
      </Tabs.Panel>
      <Tabs.Panel value='subdomains' pt='md'>
        <SubdomainsTab />
      </Tabs.Panel>
    </Tabs>
  );
}
