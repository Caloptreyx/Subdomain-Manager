import { defineEnglishItem, defineTranslations } from 'shared';

const translations = defineTranslations({
  items: {
    subdomain: defineEnglishItem('Subdomain', 'Subdomains'),
    record: defineEnglishItem('Record', 'Records'),
  },
  translations: {
    serverForm: {
      subdomainsLimit: 'Subdomains',
      subdomainsLimitDescription: 'Maximum number of subdomains this server may create.',
    },
    pages: {
      server: {
        subdomains: {
          title: 'Subdomains',
          subtitle: '{count} of {limit} subdomains used',
          empty: 'No subdomains have been created yet.',
          button: {
            create: 'Create Subdomain',
            changeAllocation: 'Change allocation',
            delete: 'Delete',
          },
          badge: {
            unknown: 'Unknown',
            primary: 'Primary',
          },
          tooltip: {
            limitReached: 'This server has reached its limit of {limit} subdomains.',
            noDomains: 'No domains are configured. Ask an administrator to add one.',
            unknownAllocation: 'Allocation was removed or the server was transferred — assign a new one.',
          },
          table: {
            columns: {
              subdomain: 'Subdomain',
              allocation: 'Allocation',
              records: 'Records',
              created: 'Created',
            },
          },
          modal: {
            create: {
              title: 'Create Subdomain',
              domain: 'Domain',
              name: 'Name',
              nameDescription: 'Lowercase letters, numbers and dashes only.',
              allocation: 'Allocation',
              preview: 'This subdomain will be reachable at {fqdn}',
            },
            changeAllocation: {
              title: 'Change Allocation',
              content: 'Choose the new allocation for {fqdn}. Its DNS records will be recreated.',
            },
            delete: {
              title: 'Delete Subdomain',
              content: 'Are you sure you want to delete **{fqdn}**? Its DNS records will be removed.',
              dnsFailed:
                'The DNS records could not be deleted: {error}\nYou can delete the subdomain anyway and leave the DNS records in place.',
              forceConfirm: 'Delete anyway (leave DNS records)',
            },
          },
          toast: {
            created: 'Subdomain created.',
            updated: 'Subdomain updated.',
            deleted: 'Subdomain deleted.',
          },
        },
      },
      admin: {
        subdomains: {
          tabs: {
            domains: 'Domains',
            settings: 'Settings',
            subdomains: 'Subdomains',
          },
          domains: {
            empty: 'No domains are configured yet.',
            button: {
              add: 'Add Domain',
              verify: 'Verify',
              edit: 'Edit',
              delete: 'Delete',
            },
            table: {
              columns: {
                domain: 'Domain',
                provider: 'Provider',
                zoneId: 'Zone ID',
                enabled: 'Enabled',
                subdomains: 'Subdomains',
                created: 'Created',
              },
            },
            provider: {
              cloudflare: 'Cloudflare',
              bunny: 'Bunny.net',
            },
            modal: {
              titleCreate: 'Add Domain',
              titleEdit: 'Edit Domain',
              form: {
                domain: 'Domain',
                provider: 'Provider',
                zoneId: 'Zone ID',
                zoneIdCloudflare: 'Zone ID from the zone overview.',
                zoneIdBunny: 'Numeric DNS zone id from the URL.',
                credential: 'API Token',
                credentialBunny: 'API Key',
                credentialKeep: 'leave empty to keep current',
              },
            },
            deleteModal: {
              title: 'Delete Domain',
              content: 'Are you sure you want to delete **{domain}**?',
              hasSubdomains:
                'This domain still has {count} subdomain(s). Deleting it will also delete all of them and their DNS records.',
              forceConfirm: 'Delete domain and all its subdomains',
            },
            toast: {
              created: 'Domain added.',
              updated: 'Domain updated.',
              deleted: 'Domain deleted.',
              verified: 'Credentials verified, zone: {zone}',
            },
          },
          settings: {
            form: {
              blacklist: 'Name Blacklist',
              blacklistDescription: 'Regular expressions matched case-insensitively against requested subdomain names.',
              blacklistInvalid: 'Invalid regular expression: {pattern}',
              defaultLimit: 'Default Subdomain Limit',
              defaultLimitDescription: 'Subdomain limit applied to new servers unless overridden.',
              defaultRecords: 'Default Record Templates',
              eggOverrides: 'Egg Overrides',
              eggOverridesDescription: 'Override the record templates for servers using a specific egg.',
              addEggOverride: 'Add egg override',
              removeEggOverride: 'Remove',
              egg: 'Egg',
            },
            templates: {
              kind: 'Type',
              kindAddress: 'Address (A/AAAA/CNAME)',
              kindSrv: 'SRV',
              kindCustom: 'Custom',
              name: 'Name',
              proxied: 'Proxied',
              ttl: 'TTL (0 = auto)',
              service: 'Service',
              protocol: 'Protocol',
              priority: 'Priority',
              weight: 'Weight',
              recordType: 'Record Type',
              content: 'Content',
              add: 'Add record template',
              remove: 'Remove record template',
              empty: 'No record templates. Subdomains will be created without DNS records.',
            },
            placeholders: {
              title: 'Template placeholders',
              content:
                'Record template fields support the placeholders below. A TTL of 0 means the provider default ("auto").',
            },
            toast: {
              saved: 'Settings saved.',
            },
            button: {
              save: 'Save',
            },
          },
          subdomains: {
            empty: 'No subdomains exist yet.',
            search: 'Search subdomains…',
            table: {
              columns: {
                subdomain: 'Subdomain',
                server: 'Server',
                allocation: 'Allocation',
                created: 'Created',
              },
            },
          },
        },
      },
    },
  },
});

export const useExtTranslations = translations.useTranslations.bind(translations);
export const getExtTranslations = translations.getTranslations.bind(translations);

export default translations;
