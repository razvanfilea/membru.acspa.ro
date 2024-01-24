import {Stack} from "@mantine/core";

export default function AdminScaffold({children}) {
    return <Stack style={{
        padding: `var(--mantine-spacing-lg)`,
        '@media (maxWidth: 900px)': {
            paddingLeft: `var(--mantine-spacing-md)`,
            paddingRight: `var(--mantine-spacing-md)`,
        },
        '@media (maxWidth: 600px)': {
            paddingLeft: 0,
            paddingRight: 0,
        }
    }}>
        {children}
    </Stack>
}