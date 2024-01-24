import 'dayjs/locale/ro';
import {ActionIcon, Button, Card, Group, Modal, Stack, Text, TextInput} from "@mantine/core";
import {useState} from "react";
import {useForm} from "@mantine/form";
import {DatePickerInput} from "@mantine/dates";
import {dateToISOString} from "../../utils/date";
import {AdminTopBar} from "../../components/AdminInput";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import useExitIfNotFounder from "../../hooks/useExitIfNotFounder";
import useFreeDaysQuery from "../../hooks/useFreeDaysQuery";
import {Database} from "../../types/database.types";
import {MdDelete} from "react-icons/md";
import AdminScaffold from "../../components/AdminInput/AdminScaffold";

export default function FreeDaysList() {
    useExitIfNotFounder();

    const supabase = useSupabaseClient<Database>()

    const {data: freeDays, refetch} = useFreeDaysQuery()
    const [createModalOpened, setCreateModalOpened] = useState(false)

    const form = useForm({
        initialValues: {
            date: new Date,
            name: null,
        },
    });

    return <>
        <Modal
            opened={createModalOpened}
            onClose={() => setCreateModalOpened(false)}
            title="Restricționează rezervarea"
        >
            <form style={{position: 'relative'}} onSubmit={
                form.onSubmit(async (values) => {
                    setCreateModalOpened(false)
                    console.log(values.date)

                    const newValue = {
                        date: dateToISOString(values.date),
                        name: values.name
                    }

                    const {error} = await supabase.from('free_days').insert([newValue])
                    console.log(error)
                    form.reset()
                    await refetch()
                })}>

                <Stack>

                    <DatePickerInput
                        {...form.getInputProps('date')}
                        label="Data"
                        withAsterisk locale="ro"
                        minDate={new Date()}
                        clearable={false}
                        size={'lg'}
                        dropdownType={'modal'}
                        valueFormat="YYYY-MM-DD"/>

                    <TextInput
                        {...form.getInputProps('name')}
                        label={'Nume'}
                        size={'lg'}
                        placeholder={'Poate lipsi'}/>

                    <Button type={"submit"} color={'green'} px={'sm'}>Adaugă</Button>
                </Stack>
            </form>
        </Modal>

        <AdminScaffold>
            <AdminTopBar
                title={'Zile libere'}
                onAdd={() => setCreateModalOpened(true)}/>

            {freeDays?.map((value) => (
                <Card key={value.date} shadow={"xs"}>
                    <Group justify="space-between">
                        <Stack gap={0}>
                            <Text>Data: <b>{(new Date(value.date)).toLocaleDateString('ro-RO')}</b></Text>

                            { value.name &&
                                <Text>Nume: <b>{value.name}</b></Text>
                            }

                            <Text mt={'xs'} size={"sm"}>Creat la {new Date(value.created_at).toLocaleString('ro-RO')}</Text>
                        </Stack>

                        <ActionIcon size={'lg'} color={'red'} variant={'filled'} onClick={async () => {
                            await supabase.from('free_days')
                                .delete()
                                .eq('date', value.date)

                            await refetch()
                        }}>
                            <MdDelete size={26}/>
                        </ActionIcon>
                    </Group>
                </Card>
            ))}
        </AdminScaffold>
    </>
}
