import {Button, Card, Group, Paper, SimpleGrid, Space, Stack, Text} from "@mantine/core";
import Link from "next/link";

export default function AdminPage() {
    return <Paper>
        <Card sx={(theme) => ({margin: theme.spacing.md})}>
            <Text size={'xl'}>Panou fondatori</Text>
            <Text size={'sm'} sx={(theme) => ({margin: theme.spacing.xs})}>
                Este necesară discreția utilizatorului: datele de intrare nu sunt validate.<br/>
                Dezvoltatorul nu își asumă nicio responsabilitate pentru această parte a aplicației!</Text>

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
            </Stack>
        </Card>
    </Paper>
}
