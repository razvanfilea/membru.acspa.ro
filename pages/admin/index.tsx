import {Button, Card, Group, Paper, SimpleGrid, Space, Stack, Text} from "@mantine/core";
import Link from "next/link";
import {useExitIfNotFounder} from "../../utils/admin_tools";

export default function AdminPage() {
    useExitIfNotFounder();

    return <Paper>
        <Card sx={(theme) => ({margin: theme.spacing.md})}>
            <Text size={'xl'}>Panou fondatori</Text>
            <Text size={'sm'} sx={(theme) => ({margin: theme.spacing.xs})}>
                Este necesară discreția utilizatorului: datele de intrare nu sunt validate.</Text>

            <Space h={'md'}/>

            <Stack spacing={'lg'}>
                <Link href={'/admin/restrictions'}>
                    <Button color={'red'}>Restricționare rezervări</Button>
                </Link>

                <Link href={'/admin/guests'}>
                    <Button color={'blue'}>Adăugare invitați</Button>
                </Link>

                <Link href={'/admin/situation'}>
                    <Button>Situație rezervări</Button>
                </Link>

                <Link href={'/admin/daily_situation'}>
                    <Button color={'pink'}>Situație zilnică</Button>
                </Link>
            </Stack>
        </Card>
    </Paper>
}
